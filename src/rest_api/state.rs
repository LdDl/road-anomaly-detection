use crate::app::{AppSettings, ZoneSettings};
use crate::frame::RawFrame;
use actix_web::web::Bytes;
use image::codecs::jpeg::JpegEncoder;
use serde::Serialize;
use std::collections::HashSet;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock, mpsc, atomic::{AtomicU64, Ordering}};
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::watch;

pub struct ZonesState {
    pub zones: Option<Vec<ZoneSettings>>,
    pub saved_zones: Option<Vec<ZoneSettings>>,
    pub revision: u64,
    pub saved_revision: u64,
}

#[derive(Clone, Default, Serialize)]
pub struct VideoInfo {
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    pub frames: u64,
    pub last_frame_at: Option<i64>,
}

pub struct ApiState {
    pub equipment_id: String,
    pub config_path: PathBuf,
    pub zones: RwLock<ZonesState>,
    pub video: Mutex<VideoInfo>,
    pub frames: watch::Sender<Option<Bytes>>,
    pub applied_revision: AtomicU64,
    preview: mpsc::SyncSender<RawFrame>,
    last_preview: Mutex<Instant>,
    preview_interval: Duration,
}

impl ApiState {
    pub fn new(settings: &AppSettings, config_path: &str) -> io::Result<Arc<Self>> {
        if settings.rest_api.preview_fps == 0 || settings.rest_api.preview_fps > 30 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "rest_api.preview_fps must be between 1 and 30"));
        }
        let config_path = std::fs::canonicalize(config_path)?;
        let (preview, receiver) = mpsc::sync_channel::<RawFrame>(1);
        let (frames, _) = watch::channel(None);
        let broadcaster = frames.clone();
        thread::Builder::new().name("web-preview".to_string()).spawn(move || {
            for mut frame in receiver {
                if broadcaster.receiver_count() == 0 {
                    continue;
                }
                for pixel in frame.data.chunks_exact_mut(3) {
                    pixel.swap(0, 2);
                }
                let mut jpeg = Vec::new();
                if let Err(err) = JpegEncoder::new_with_quality(&mut jpeg, 80).encode(&frame.data, frame.width, frame.height, image::ExtendedColorType::Rgb8) {
                    eprintln!("Can't encode web preview: {}", err);
                    continue;
                }
                broadcaster.send_replace(Some(Bytes::from(jpeg)));
            }
        })?;
        Ok(Arc::new(Self {
            equipment_id: settings.application_info.id.clone(),
            config_path,
            zones: RwLock::new(ZonesState {
                zones: settings.zones.clone(),
                saved_zones: settings.zones.clone(),
                revision: 0,
                saved_revision: 0,
            }),
            video: Mutex::new(VideoInfo::default()),
            frames,
            applied_revision: AtomicU64::new(0),
            preview,
            last_preview: Mutex::new(Instant::now()),
            preview_interval: Duration::from_secs_f64(1.0 / settings.rest_api.preview_fps as f64),
        }))
    }

    pub fn set_video_info(&self, width: u32, height: u32, fps: f32) {
        let mut video = self.video.lock().unwrap();
        video.width = width;
        video.height = height;
        video.fps = fps;
    }

    pub fn publish_frame(&self, frame: &RawFrame) {
        {
            let mut video = self.video.lock().unwrap();
            video.frames += 1;
            video.last_frame_at = Some(chrono::Utc::now().timestamp_millis());
        }
        if self.frames.receiver_count() == 0 {
            return;
        }
        let mut last_preview = self.last_preview.lock().unwrap();
        if last_preview.elapsed() >= self.preview_interval {
            // A slow browser or JPEG encoder must not hold up detection.
            let _ = self.preview.try_send(frame.clone());
            *last_preview = Instant::now();
        }
    }

    pub fn zones_since(&self, revision: u64) -> Option<(u64, Option<Vec<ZoneSettings>>)> {
        let zones = self.zones.read().unwrap();
        if zones.revision == revision {
            None
        } else {
            Some((zones.revision, zones.zones.clone()))
        }
    }

    pub fn mark_applied(&self, revision: u64) {
        self.applied_revision.store(revision, Ordering::Release);
    }

    pub fn validate_zones(&self, zones: &[ZoneSettings]) -> Result<(), String> {
        let video = self.video.lock().unwrap();
        validate_zones(zones, video.width, video.height)
    }
}

pub fn validate_zones(zones: &[ZoneSettings], width: u32, height: u32) -> Result<(), String> {
    if zones.len() > 256 {
        return Err("At most 256 zones are supported".to_string());
    }
    let mut identifiers = HashSet::new();
    for zone in zones {
        if zone.id.trim().is_empty() || zone.id.len() > 128 || zone.id.chars().any(char::is_control) || !identifiers.insert(&zone.id) {
            return Err("Zone IDs must be unique, nonempty and at most 128 bytes long".to_string());
        }
        if zone.color_rgb.map_or(false, |rgb| rgb.iter().any(|value| *value > 255)) {
            return Err(format!("Zone {}: RGB values must be between 0 and 255", zone.id));
        }
        let points = &zone.geometry;
        for (index, [x, y]) in points.iter().enumerate() {
            if *x < 0 || *y < 0 || (width > 0 && *x as u32 >= width) || (height > 0 && *y as u32 >= height) {
                return Err(format!("Zone {}: coordinates must be inside the original frame", zone.id));
            }
            if points[..index].contains(&[*x, *y]) {
                return Err(format!("Zone {}: all four vertices must be distinct", zone.id));
            }
        }
        let area: i128 = (0..4).map(|i| {
            let next = (i + 1) % 4;
            points[i][0] as i128 * points[next][1] as i128 - points[next][0] as i128 * points[i][1] as i128
        }).sum();
        if area == 0 || segments_intersect(points[0], points[1], points[2], points[3]) || segments_intersect(points[1], points[2], points[3], points[0]) {
            return Err(format!("Zone {}: polygon must have an area and no intersecting edges", zone.id));
        }
    }
    Ok(())
}

fn segments_intersect(a: [i32; 2], b: [i32; 2], c: [i32; 2], d: [i32; 2]) -> bool {
    fn cross(a: [i32; 2], b: [i32; 2], c: [i32; 2]) -> i128 {
        (b[0] as i128 - a[0] as i128) * (c[1] as i128 - a[1] as i128) - (b[1] as i128 - a[1] as i128) * (c[0] as i128 - a[0] as i128)
    }
    fn on_segment(a: [i32; 2], b: [i32; 2], p: [i32; 2]) -> bool {
        p[0] >= a[0].min(b[0]) && p[0] <= a[0].max(b[0]) && p[1] >= a[1].min(b[1]) && p[1] <= a[1].max(b[1])
    }
    let (ab_c, ab_d, cd_a, cd_b) = (cross(a, b, c), cross(a, b, d), cross(c, d, a), cross(c, d, b));
    (ab_c.signum() * ab_d.signum() < 0 && cd_a.signum() * cd_b.signum() < 0) ||
        (ab_c == 0 && on_segment(a, b, c)) || (ab_d == 0 && on_segment(a, b, d)) ||
        (cd_a == 0 && on_segment(c, d, a)) || (cd_b == 0 && on_segment(c, d, b))
}

use super::{ApiState, ZonesState, config};
use crate::app::{AppSettings, ZoneSettings};
use actix_web::{http::StatusCode, web, HttpResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::atomic::Ordering;

#[derive(Serialize)]
pub struct ZonesResponse {
    pub zones: Vec<ZoneSettings>,
    pub whole_frame: bool,
    pub revision: u64,
    pub saved_revision: u64,
}

#[derive(Deserialize)]
pub struct ZonesRequest {
    pub zones: Vec<ZoneSettings>,
    #[serde(default)]
    pub whole_frame: bool,
    pub revision: u64,
}

#[derive(Deserialize)]
pub struct ZoneRequest {
    pub zone: ZoneSettings,
    pub revision: u64,
}

#[derive(Deserialize)]
pub struct ZoneUpdateRequest {
    pub zone_id: String,
    pub zone: ZoneSettings,
    pub revision: u64,
}

#[derive(Deserialize)]
pub struct ZoneDeleteRequest {
    pub zone_id: String,
    pub revision: u64,
}

#[derive(Deserialize)]
pub struct SaveRequest {
    pub revision: u64,
}

pub fn error(status: StatusCode, message: impl Into<String>) -> HttpResponse {
    HttpResponse::build(status).json(json!({"error": message.into()}))
}

fn snapshot(zones: &ZonesState) -> ZonesResponse {
    ZonesResponse {
        zones: zones.zones.clone().unwrap_or_default(),
        whole_frame: zones.zones.is_none(),
        revision: zones.revision,
        saved_revision: zones.saved_revision,
    }
}

pub async fn ping() -> HttpResponse {
    HttpResponse::Ok().json(json!({"pong": true}))
}

pub async fn health(state: web::Data<ApiState>) -> HttpResponse {
    let video = state.video.lock().unwrap().clone();
    let status = match video.last_frame_at {
        None => "waiting_for_video",
        Some(time) if chrono::Utc::now().timestamp_millis() - time > 10000 => "stale",
        Some(_) => "streaming",
    };
    let zones = state.zones.read().unwrap();
    HttpResponse::Ok().json(json!({
        "status": status,
        "equipment_id": state.equipment_id,
        "version": env!("CARGO_PKG_VERSION"),
        "video": video,
        "revision": zones.revision,
        "applied_revision": state.applied_revision.load(Ordering::Acquire),
        "saved_revision": zones.saved_revision,
    }))
}

pub async fn get_zones(state: web::Data<ApiState>) -> HttpResponse {
    HttpResponse::Ok().json(snapshot(&state.zones.read().unwrap()))
}

pub async fn update_zones(state: web::Data<ApiState>, body: web::Json<ZonesRequest>) -> HttpResponse {
    if body.whole_frame && !body.zones.is_empty() {
        return error(StatusCode::BAD_REQUEST, "whole_frame requires an empty zones list");
    }
    mutate_zones(&state, body.revision, |_| {
        Ok(if body.whole_frame { None } else { Some(body.zones.clone()) })
    })
}

pub async fn create_zone(state: web::Data<ApiState>, body: web::Json<ZoneRequest>) -> HttpResponse {
    mutate_zones(&state, body.revision, |current| {
        let mut zones = current.clone().unwrap_or_default();
        zones.push(body.zone.clone());
        Ok(Some(zones))
    })
}

pub async fn update_zone(state: web::Data<ApiState>, body: web::Json<ZoneUpdateRequest>) -> HttpResponse {
    mutate_zones(&state, body.revision, |current| {
        let mut zones = current.clone().unwrap_or_default();
        let zone = zones.iter_mut().find(|zone| zone.id == body.zone_id).ok_or_else(|| "Zone not found".to_string())?;
        *zone = body.zone.clone();
        Ok(Some(zones))
    })
}

pub async fn delete_zone(state: web::Data<ApiState>, body: web::Json<ZoneDeleteRequest>) -> HttpResponse {
    mutate_zones(&state, body.revision, |current| {
        let mut zones = current.clone().unwrap_or_default();
        let index = zones.iter().position(|zone| zone.id == body.zone_id).ok_or_else(|| "Zone not found".to_string())?;
        zones.remove(index);
        Ok(Some(zones))
    })
}

fn mutate_zones<F>(state: &ApiState, revision: u64, mutation: F) -> HttpResponse
where F: FnOnce(&Option<Vec<ZoneSettings>>) -> Result<Option<Vec<ZoneSettings>>, String> {
    let mut current = state.zones.write().unwrap();
    if current.revision != revision {
        return error(StatusCode::CONFLICT, "Zones were changed by another client. Reload before editing");
    }
    let zones = match mutation(&current.zones) {
        Ok(zones) => zones,
        Err(message) => return error(StatusCode::NOT_FOUND, message),
    };
    if let Some(zones) = &zones {
        if let Err(message) = state.validate_zones(zones) {
            return error(StatusCode::BAD_REQUEST, message);
        }
    }
    if current.zones != zones {
        current.zones = zones;
        current.revision += 1;
    }
    HttpResponse::Ok().json(snapshot(&current))
}

pub async fn save_config(state: web::Data<ApiState>, body: web::Json<SaveRequest>) -> HttpResponse {
    let revision = body.revision;
    let result = web::block(move || {
        let mut zones = state.zones.write().unwrap();
        if zones.revision != revision {
            return Err((StatusCode::CONFLICT, "Zones changed before saving. Reload before editing".to_string()));
        }
        let contents = std::fs::read_to_string(&state.config_path).map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
        let disk: AppSettings = toml::from_str(&contents).map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
        if disk.zones != zones.saved_zones {
            return Err((StatusCode::CONFLICT, "Zones in the TOML file were modified outside this application. Restart to load them".to_string()));
        }
        let replacement = config::update_document(&contents, &zones.zones).map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err))?;
        let backup = config::save_document(&state.config_path, &replacement).map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, format!("Can't save configuration: {}. The configuration directory must be writable", err)))?;
        zones.saved_revision = zones.revision;
        zones.saved_zones = zones.zones.clone();
        Ok(json!({"message": "Configuration saved", "backup": backup, "revision": zones.revision}))
    }).await;
    match result {
        Ok(Ok(response)) => HttpResponse::Ok().json(response),
        Ok(Err((status, message))) => error(status, message),
        Err(err) => error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

pub async fn mjpeg_stream(state: web::Data<ApiState>) -> HttpResponse {
    if state.frames.receiver_count() >= 16 {
        return error(StatusCode::SERVICE_UNAVAILABLE, "Too many preview clients");
    }
    let mut receiver = state.frames.subscribe();
    receiver.mark_changed();
    let stream = futures_util::stream::unfold(receiver, |mut receiver| async move {
        loop {
            receiver.changed().await.ok()?;
            let frame = receiver.borrow_and_update().clone();
            if let Some(frame) = frame {
                let mut chunk = format!("--frame\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n", frame.len()).into_bytes();
                chunk.extend_from_slice(&frame);
                chunk.extend_from_slice(b"\r\n");
                return Some((Ok::<_, actix_web::Error>(web::Bytes::from(chunk)), receiver));
            }
        }
    });
    HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-store"))
        .insert_header(("X-Accel-Buffering", "no"))
        .content_type("multipart/x-mixed-replace; boundary=frame")
        .streaming(stream)
}

pub async fn frame(state: web::Data<ApiState>) -> HttpResponse {
    let mut receiver = state.frames.subscribe();
    let latest = receiver.borrow().clone();
    let latest = if latest.is_some() {
        latest
    } else if actix_web::rt::time::timeout(std::time::Duration::from_secs(5), receiver.changed()).await.is_ok() {
        receiver.borrow().clone()
    } else {
        None
    };
    match latest {
        Some(frame) => HttpResponse::Ok().insert_header(("Cache-Control", "no-store")).content_type("image/jpeg").body(frame),
        None => error(StatusCode::SERVICE_UNAVAILABLE, "No video frame available yet"),
    }
}

pub async fn openapi() -> HttpResponse {
    HttpResponse::Ok().content_type("application/json").body(include_str!("openapi.json"))
}

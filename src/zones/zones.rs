use crate::tracker::Tracker;
use crate::events::{EventInfo, EventBBox, EventPOI};

use uuid::Uuid;
use chrono::Utc;
use opencv::{
    core::Mat, core::Point2f, core::Point2i, core::Scalar, core::Vector, imgproc::line, imgproc::point_polygon_test, imgproc::LINE_8
};

use std::collections::HashSet;

#[derive(Debug)]
pub struct Zone {
    pub id: String,
    pub color: Scalar,
    pixel_coordinates: Vector<Point2f>,
    segments: [[Point2i; 2]; 4],
    objects_registered: HashSet<Uuid>
}

impl Zone {
    pub fn new(id: String, coordinates: [[i32; 2]; 4], color_rgb: Option<[u16; 3]>) -> Self {
        let pixel_coordinates: Vector<Point2f> = coordinates.iter().map(|pair| {
            Point2f::new(pair[0] as f32, pair[1] as f32)
        }).collect();
        let mut segments: [[Point2i; 2]; 4] = [[Point2i::new(0, 0), Point2i::new(0, 0)], [Point2i::new(0, 0), Point2i::new(0, 0)], [Point2i::new(0, 0), Point2i::new(0, 0)], [Point2i::new(0, 0), Point2i::new(0, 0)]];
        for i in 1..coordinates.len() {
            let prev_pt = Point2i::new(
                coordinates[i - 1][0],
                coordinates[i - 1][1],
            );
            let current_pt = Point2i::new(
                coordinates[i][0],
                coordinates[i][1],
            );
            segments[i-1] = [prev_pt, current_pt];
        }
        segments[segments.len() - 1] = [Point2i::new(coordinates[coordinates.len()-1][0], coordinates[coordinates.len()-1][1]), Point2i::new(coordinates[0][0], coordinates[0][1])];
        let color = match color_rgb {
            Some(rgb_array) => Scalar::from((rgb_array[2] as f64, rgb_array[1] as f64, rgb_array[0] as f64)),
            None => Scalar::from((0., 0., 0.))
        };
        Zone{
            id,
            color,
            pixel_coordinates,
            segments,
            objects_registered: HashSet::new()
        }
    }
    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        let ppt = point_polygon_test(&self.pixel_coordinates, Point2f::new(x, y), false).unwrap();
        ppt > 0.0 
    }
    pub fn draw(&self, img: &mut Mat) {
        for seg in self.segments {
            line(img, seg[0], seg[1], self.color, 2, LINE_8, 0).unwrap();
        } 
    }
    pub fn process_tracker(&mut self, tracker: &mut Tracker, min_lifetime_seconds: i64, max_lifetime_seconds: i64, app_id: Option<String>, frame: Option<&Mat>) -> Vec<EventInfo> {
        let mut new_events: Vec<EventInfo> = vec![];
        let current_ut = Utc::now().timestamp();
        for (object_id, object) in tracker.engine.objects.iter() {
            // Filter objects which disappeared in current time
            if object.get_no_match_times() > 1 {
                continue;
            }
            let center = object.get_center();
            let object_extra = tracker.objects_extra.get(object_id);
            if object_extra.is_none() {
                continue;
            }
            let object_extra = object_extra.unwrap();
            // Filter objects by min lifetime threshold
            let object_lifetime = object_extra.get_lifetime();
            if object_lifetime <= min_lifetime_seconds {
                continue;
            }
            if self.contains_point(center.x, center.y) {
                if self.objects_registered.contains(object_id) {
                    if object_lifetime > max_lifetime_seconds {
                        tracker.objects_extra.remove(object_id); // Remove object from tracker data to make it appear in next iteration again if object still exist
                        self.objects_registered.remove(object_id);
                    }
                    continue;
                }
                self.objects_registered.insert(*object_id);
                // Prepare event_info
                let bbox = object.get_bbox();
                let center = object.get_center();
                let new_event = EventInfo::new(
                    current_ut,
                    frame,
                    object_id.to_string(),
                    object_extra.get_register_time(),
                    object_lifetime,
                    EventBBox{
                        x: bbox.x.floor() as i32,
                        y: bbox.y.floor() as i32,
                        width: bbox.width.floor() as i32,
                        height: bbox.height.floor() as i32
                    },
                    EventPOI{
                        x: center.x.floor() as i32,
                        y: center.y.floor() as i32
                    },
                    self.id.clone(),
                    app_id.clone(),
                );
                new_events.push(new_event);
            }
        }
        new_events
    }
}

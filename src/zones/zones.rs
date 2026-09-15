use crate::tracker::Tracker;
use crate::events::{EventInfo, EventBBox, EventPOI};
use crate::frame::RawFrame;
use crate::draw::{Scalar, draw_line};

use uuid::Uuid;
use chrono::Utc;

use std::collections::HashSet;

#[derive(Debug)]
pub struct Zone {
    pub id: String,
    pub color: Scalar,
    pixel_coordinates: [[i32; 2]; 4],
    segments: [[[i32; 2]; 2]; 4],
    objects_registered: HashSet<Uuid>
}

impl Zone {
    pub fn new(id: String, coordinates: [[i32; 2]; 4], color_rgb: Option<[u16; 3]>) -> Self {
        let pixel_coordinates = coordinates;
        let mut segments = [[[0; 2]; 2]; 4];
        for i in 1..coordinates.len() {
            let prev_pt = coordinates[i - 1];
            let current_pt = coordinates[i];
            segments[i-1] = [prev_pt, current_pt];
        }
        segments[segments.len() - 1] = [coordinates[coordinates.len()-1], coordinates[0]];
        let color = match color_rgb {
            Some(rgb_array) => Scalar::from((rgb_array[2].min(255) as u8, rgb_array[1].min(255) as u8, rgb_array[0].min(255) as u8)),
            None => Scalar::from((0, 0, 0))
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
        let x = x as f64;
        let y = y as f64;
        let mut inside = false;
        let mut previous = self.pixel_coordinates.len() - 1;
        for current in 0..self.pixel_coordinates.len() {
            let [ax, ay] = self.pixel_coordinates[previous].map(|value| value as f64);
            let [bx, by] = self.pixel_coordinates[current].map(|value| value as f64);
            let cross = (x - ax) * (by - ay) - (y - ay) * (bx - ax);
            // The boundary is excluded, matching the original zone check.
            if cross == 0.0 && x >= ax.min(bx) && x <= ax.max(bx) && y >= ay.min(by) && y <= ay.max(by) {
                return false;
            }
            if (ay > y) != (by > y) && x < ax + (y - ay) * (bx - ax) / (by - ay) {
                inside = !inside;
            }
            previous = current;
        }
        inside
    }
    pub fn draw(&self, img: &mut RawFrame) {
        for seg in self.segments {
            draw_line(img, seg[0], seg[1], self.color, 2);
        } 
    }
    pub fn process_tracker(&mut self, tracker: &mut Tracker, min_lifetime_seconds: i64, max_lifetime_seconds: i64, app_id: Option<String>, frame: Option<&RawFrame>) -> Vec<EventInfo> {
        let mut new_events: Vec<EventInfo> = vec![];
        let current_ut = Utc::now().timestamp();
        for (object_id, object) in tracker.engine.objects.iter() {
            // Filter objects which disappeared in current time
            if object.get_no_match_times() > 1 {
                continue;
            }
            let center = object.get_center();
            let object_extra = match tracker.objects_extra.get(object_id) {
                Some(extra) => extra,
                None => continue,
            };
            let object_extra = object_extra;
            // Filter objects by min lifetime threshold
            let object_lifetime = object_extra.get_lifetime();
            if object_lifetime <= min_lifetime_seconds {
                continue;
            }
            let contains_object = self.contains_point(center.x, center.y);
            if contains_object {
                if self.objects_registered.contains(object_id) {
                    if object_lifetime > max_lifetime_seconds {
                        // Remove object from tracker data to make it appear in next iteration again if object still exist
                        tracker.objects_extra.remove(object_id);
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
                    object_extra.get_classname(),
                    object_extra.get_confidence(),
                    self.id.clone(),
                    app_id.clone(),
                );
                new_events.push(new_event);
            }
        }
        new_events
    }
}

use crate::detection::Detections;

use uuid::Uuid;
use mot_rs::mot::{
    IoUTracker
};

use std::error::Error;
use std::collections::HashMap;
use std::collections::hash_map::Entry::{
    Occupied,
    Vacant
};
use std::fmt;

pub struct ObjectExtra {
    class_name: String,
    confidence: f32,
}

impl ObjectExtra {
    pub fn get_classname(&self) -> String {
        self.class_name.clone()
    }
}

pub struct Tracker {
    pub engine: IoUTracker,
    pub objects_extra: HashMap<Uuid, ObjectExtra>,
}

impl fmt::Display for Tracker {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.engine)
    }
}

impl Tracker {
    pub fn new(_max_no_match: usize, _iou_threshold: f32) -> Self {
        Self {
            engine: IoUTracker::new(_max_no_match, _iou_threshold),
            objects_extra: HashMap::new(),
        }
    }
    pub fn match_objects(&mut self, detections: &mut Detections, current_second: f32) -> Result<(), Box<dyn Error>> {
        self.engine.match_objects(&mut detections.blobs)?;
        for (idx, detection) in detections.blobs.iter().enumerate() {
            let object_id = detection.get_id();
            match self.objects_extra.entry(object_id) {
                Occupied(mut entry) => {
                    todo!("occupied");
                },
                Vacant(entry) => {
                    todo!("vacant");
                }
            }
        }
        Ok(())
    }
}



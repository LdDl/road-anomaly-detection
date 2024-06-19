use uuid::Uuid;
use mot_rs::mot::{
    IoUTracker
};

use std::collections::HashMap;
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

impl Tracker {
    pub fn new(_max_no_match: usize, _iou_threshold: f32) -> Self {
        Self {
            engine: IoUTracker::new(_max_no_match, _iou_threshold),
            objects_extra: HashMap::new(),
        }
    }
}

impl fmt::Display for Tracker {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.engine)
    }
}


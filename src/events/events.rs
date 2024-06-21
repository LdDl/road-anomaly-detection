use crate::utils::serialize_mat_as_base64;

use serde::Serialize;
use uuid::Uuid;
use opencv::core::Mat;

#[derive(Debug, Serialize)]
pub struct EventBBox {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32
}

#[derive(Debug, Serialize)]
pub struct EventPOI {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Serialize)]
pub struct EventInfo {
    id: Uuid,
    event_registered_at: i64,
    #[serde(serialize_with = "serialize_mat_as_base64")]
    event_image: Option<Mat>,
    object_id: String,
    object_registered_at: i64,
    object_lifetime: i64,
    object_bbox: EventBBox,
    object_poi: EventPOI,
    object_classname: String,
    object_confidence: f32,
    zone_id: String,
    equipment_id: Option<String>
}

impl EventInfo{
    pub fn new(unix_tm: i64, frame: Option<&Mat>, object_id: String, object_registered_unix_tm: i64, object_lifetime: i64, object_bbox: EventBBox, object_poi: EventPOI, classname: String, confidence: f32, zone_id: String, equipment_id: Option<String>) -> Self {
        EventInfo{
            id: Uuid::new_v4(),
            event_registered_at: unix_tm,
            // event_image: frame.map(|img| img.clone()),
            event_image: frame.cloned(),
            object_id,
            object_registered_at: object_registered_unix_tm,
            object_lifetime,
            object_bbox, 
            object_poi,
            object_classname: classname,
            object_confidence: confidence,
            zone_id,
            equipment_id 
        }
    }
    pub fn get_id(&self) -> Uuid {
        self.id
    }
}


use opencv::core::Rect as RectCV;

use mot_rs::mot::SimpleBlob;
use mot_rs::utils::{
    Rect, Point
};

use std::collections::HashSet;

#[derive(Debug)]
pub struct Detections {
    pub blobs: Vec<SimpleBlob>,
    pub class_names: Vec<String>,
    pub confidences: Vec<f32>,
}

pub fn process_yolo_detections(nms_bboxes: &Vec<RectCV>, nms_classes_ids: Vec<usize>, nms_confidences: Vec<f32>, net_classes: &[String], target_classes: &HashSet<String>, dt: f32, scale_width: f32, scale_height: f32) -> Detections {
    if (nms_bboxes.len() != nms_classes_ids.len()) || (nms_bboxes.len() != nms_confidences.len()) || (nms_classes_ids.len() != nms_confidences.len()) {
        // Something wrong?
        println!("BBoxes len: {}, Classed IDs len: {}, Confidences len: {}", nms_bboxes.len(), nms_classes_ids.len(), nms_confidences.len());
        return Detections {
            blobs: vec![],
            class_names: vec![],
            confidences: vec![]
        };
    }
    let mut aggregated_data = vec![];
    let mut class_names: Vec<String> = Vec::with_capacity(nms_classes_ids.len());
    for (i, bbox) in nms_bboxes.iter().enumerate() {
        let class_id = nms_classes_ids[i];
        if class_id >= net_classes.len() {
            // Evade panic?
            continue
        };
        let classname = net_classes[class_id].clone();
        if !target_classes.is_empty() && !target_classes.contains(&classname) {
            continue;
        }
        class_names.push(classname);
        let center_x = (bbox.x as f32 + bbox.width as f32 / 2.0) * scale_width;
        let center_y = (bbox.y as f32 + bbox.height as f32 / 2.0) * scale_height;
        let kb: SimpleBlob = SimpleBlob::new_with_center_dt(Point::new(center_x, center_y), Rect::new(bbox.x as f32 * scale_width, bbox.y as f32 * scale_height, bbox.width as f32 * scale_width, bbox.height as f32 * scale_height), dt);
        // let mut kb = SimpleBlob::new_with_dt(Rect::new(bbox.x as f32, bbox.y as f32, bbox.width as f32, bbox.height as f32), dt);
        aggregated_data.push(kb);
    }
    Detections {
        blobs: aggregated_data,
        class_names,
        confidences: nms_confidences,
    }
}

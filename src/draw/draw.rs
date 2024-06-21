// Taken from https://github.com/LdDl/rust-road-traffic/blob/master/src/lib/draw/draw.rs
use opencv::{
    core::Mat,
    core::Rect,
    core::Point,
    core::Scalar,
    imgproc::LINE_8,
    imgproc::LINE_4,
    imgproc::FONT_HERSHEY_SIMPLEX,
    imgproc::rectangle,
    imgproc::put_text,
};
use crate::tracker::Tracker;

pub fn invert_color(color: &Scalar) -> Scalar {
    let b = color[0];
    let g = color[1];
    let r = color[2];
    let inv_b = 255.0 - b;
    let inv_g = 255.0 - g;
    let inv_r = 255.0 - r;
    Scalar::from((inv_b, inv_g, inv_r))
}

pub fn draw_bboxes(img: &mut Mat, tracker: &Tracker, color: Scalar, inv_color: Scalar) {
    for (_, object) in tracker.engine.objects.iter() {
        let mut color_choose = color;
        if object.get_no_match_times() > 1 {
            color_choose = inv_color;
        }
        let bbox = object.get_bbox();
        let cv_rect = Rect::new(bbox.x.floor() as i32, bbox.y.floor() as i32, bbox.width as i32, bbox.height as i32);
        match rectangle(img, cv_rect, color_choose, 2, LINE_4, 0) {
            Ok(_) => {},
            Err(err) => {
                eprintln!("Can't draw rectangle at blob's bbox due the error: {:?}", err)
            }
        };
    }
}

pub fn draw_identifiers(img: &mut Mat, tracker: &Tracker, color: Scalar, inv_color: Scalar) {
    for (_, object) in tracker.engine.objects.iter() {
        let mut color_choose = color;
        if object.get_no_match_times() > 1 {
            color_choose = inv_color;
        }
        let bbox = object.get_bbox();
        let anchor = Point::new(bbox.x.floor() as i32 + 2, bbox.y.floor() as i32 + 10);
        match put_text(img, &object.get_id().to_string(), anchor, FONT_HERSHEY_SIMPLEX, 0.5, color_choose, 2, LINE_8, false) {
            Ok(_) => {},
            Err(err) => {
                eprintln!("Can't display ID of object due the error {:?}", err);
            }
        };
    }
}

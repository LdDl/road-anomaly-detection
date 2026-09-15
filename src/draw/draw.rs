use crate::frame::RawFrame;
use crate::draw::primitives::{draw_line, draw_text};
use crate::tracker::Tracker;

#[derive(Debug, Clone, Copy)]
pub struct Scalar(pub [u8; 3]);

impl From<(u8, u8, u8)> for Scalar {
    fn from((b, g, r): (u8, u8, u8)) -> Self {
        Scalar([b, g, r])
    }
}

impl std::ops::Index<usize> for Scalar {
    type Output = u8;
    fn index(&self, index: usize) -> &u8 {
        &self.0[index]
    }
}

pub fn invert_color(color: &Scalar) -> Scalar {
    let b = color[0];
    let g = color[1];
    let r = color[2];
    let inv_b = 255 - b;
    let inv_g = 255 - g;
    let inv_r = 255 - r;
    Scalar::from((inv_b, inv_g, inv_r))
}

pub fn draw_bboxes(img: &mut RawFrame, tracker: &Tracker, color: Scalar, inv_color: Scalar) {
    for (_, object) in tracker.engine.objects.iter() {
        let mut color_choose = color;
        if object.get_no_match_times() > 1 {
            color_choose = inv_color;
        }
        let bbox = object.get_bbox();
        let left = bbox.x.floor() as i32;
        let top = bbox.y.floor() as i32;
        let right = (bbox.x + bbox.width).floor() as i32;
        let bottom = (bbox.y + bbox.height).floor() as i32;
        draw_line(img, [left, top], [right, top], color_choose, 2);
        draw_line(img, [right, top], [right, bottom], color_choose, 2);
        draw_line(img, [right, bottom], [left, bottom], color_choose, 2);
        draw_line(img, [left, bottom], [left, top], color_choose, 2);
    }
}

pub fn draw_identifiers(img: &mut RawFrame, tracker: &Tracker, color: Scalar, inv_color: Scalar) {
    for (_, object) in tracker.engine.objects.iter() {
        let mut color_choose = color;
        if object.get_no_match_times() > 1 {
            color_choose = inv_color;
        }
        let bbox = object.get_bbox();
        let anchor = [bbox.x.floor() as i32 + 2, bbox.y.floor() as i32];
        draw_text(img, anchor, &object.get_id().to_string(), color_choose);
    }
}

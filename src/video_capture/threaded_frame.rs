// Copy of https://github.com/LdDl/rust-road-traffic/blob/master/src/video_capture/frame.rs
use crate::frame::RawFrame;

pub struct ThreadedFrame {
    pub frame: RawFrame,
    pub overall_seconds: f32,
    pub current_second: f32
}

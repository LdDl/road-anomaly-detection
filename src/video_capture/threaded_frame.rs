// Copy of https://github.com/LdDl/rust-road-traffic/blob/master/src/video_capture/frame.rs
use opencv::core::Mat;

pub struct ThreadedFrame {
    pub frame: Mat,
    pub overall_seconds: f32,
    pub current_second: f32
}

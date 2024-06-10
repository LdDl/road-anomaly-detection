// Copy of https://github.com/LdDl/rust-road-traffic/blob/master/src/video_capture/mod.rs
mod threaded_frame;
mod video_capture;

pub use self::{threaded_frame::*, video_capture::*};
pub mod app;
pub mod utils;
pub mod video_capture;
pub mod detection;
pub mod tracker;
pub mod events;
pub mod zones;
pub mod draw;
pub mod publisher;
pub mod frame;
pub mod background;

#[cfg(not(feature = "ort-backend"))]
compile_error!("Enable the ort-backend or ort-cuda feature");

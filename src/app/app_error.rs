use crate::video_capture;
use std::fmt;
use toml;

#[derive(Debug)]
pub struct AppInternalError{pub typ: i16, pub txt: String}
impl fmt::Display for AppInternalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.typ {
            1 => write!(f, "Invalid device identifier"),
            2 => write!(f, "Video has not been opened"),
            3 => write!(f, "Bad model format: '{}'", self.txt),
            4 => write!(f, "Bad model version: '{}'", self.txt),
            5 => write!(f, "Bad tracker parameters: '{}'", self.txt),
            6 => write!(f, "Bad image dimensions: '{}'", self.txt),
            _ => write!(f, "Undefined VideoCapture error")
        }
    }
}

#[derive(Debug)]
pub enum AppError {
    Internal(AppInternalError),
    VideoError(video_capture::VideoCaptureError),
    ModelError(od_opencv::OrtModelError),
    WindowError(minifb::Error),
    SignalError(ctrlc::Error),
    TOMLError(toml::de::Error),
}

impl From<AppInternalError> for AppError {
    fn from(e: AppInternalError) -> Self {
        AppError::Internal(e)
    }
}

impl From<video_capture::VideoCaptureError> for AppError {
    fn from(e: video_capture::VideoCaptureError) -> Self {
        AppError::VideoError(e)
    }
}

impl From<od_opencv::OrtModelError> for AppError {
    fn from(e: od_opencv::OrtModelError) -> Self {
        AppError::ModelError(e)
    }
}

impl From<toml::de::Error> for AppError {
    fn from(e: toml::de::Error) -> Self {
        AppError::TOMLError(e)
    }
}

impl From<minifb::Error> for AppError {
    fn from(e: minifb::Error) -> Self {
        AppError::WindowError(e)
    }
}

impl From<ctrlc::Error> for AppError {
    fn from(e: ctrlc::Error) -> Self {
        AppError::SignalError(e)
    }
}

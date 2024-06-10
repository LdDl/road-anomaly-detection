// Copy of https://github.com/LdDl/rust-road-traffic/blob/master/src/video_capture/video_capture.rs
use std::fmt;

use opencv::{
    videoio::VideoCapture,
    videoio::CAP_ANY,
};

#[derive(Debug)]
struct VideoCaptureInternalError{typ: i16}
impl fmt::Display for VideoCaptureInternalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.typ {
            1 => write!(f, "Invalid device identifier"),
            _ => write!(f, "Undefined VideoCapture error")
        }
    }
}

#[derive(Debug)]
pub enum VideoCaptureError {
    VideoError(VideoCaptureInternalError),
    OpenCVError(opencv::Error),
}

impl fmt::Display for VideoCaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VideoCaptureError::VideoError(e) => write!(f, "{}", e),
            VideoCaptureError::OpenCVError(e) => write!(f, "{}", e),
        }
    }
}

impl From<VideoCaptureInternalError> for VideoCaptureError {
    fn from(e: VideoCaptureInternalError) -> Self {
        VideoCaptureError::VideoError(e)
    }
}

impl From<opencv::Error> for VideoCaptureError {
    fn from(e: opencv::Error) -> Self {
        VideoCaptureError::OpenCVError(e)
    }
}

pub fn get_video_capture(video_src: &str, typ: String) -> Result<VideoCapture, VideoCaptureError> {
    if typ == "rtsp" {
        let video_capture = match VideoCapture::from_file(video_src, CAP_ANY) {
            Ok(result) => {result},
            Err(err) => {
                return Err(VideoCaptureError::OpenCVError(err))
            }
        };
        return Ok(video_capture);
    }
    let device_id = match video_src.parse::<i32>() {
        Ok(result) => {result},
        Err(err) => {
            return Err(VideoCaptureError::VideoError(VideoCaptureInternalError{typ: 1}))
        }
        Err(err) => {
            panic!("Can't parse '{}' as device_id (i32) due the error: {:?}", video_src, err);
        }
    };
    let video_capture = match VideoCapture::new(device_id, CAP_ANY) {
        Ok(result) => {result},
        Err(err) => {
            return Err(VideoCaptureError::OpenCVError(err))
        }
    };
    return Ok(video_capture);
}
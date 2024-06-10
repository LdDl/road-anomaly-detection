use crate::video_capture;
use std::fmt;
use std::fs;
use toml;
use serde::{ Deserialize, Serialize };
use opencv::{
    videoio::VideoCapture,
    prelude::VideoCaptureTrait,
    prelude::VideoCaptureTraitConst,
    prelude::MatTraitConst,
    core::Mat,
};

const EMPTY_FRAMES_LIMIT: u16 = 60;

#[derive(Debug)]
pub struct AppInternalError{typ: i16}
impl fmt::Display for AppInternalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.typ {
            1 => write!(f, "Invalid device identifier"),
            2 => write!(f, "Video has not been opened"),
            _ => write!(f, "Undefined VideoCapture error")
        }
    }
}

#[derive(Debug)]
pub enum AppError {
    Internal(AppInternalError),
    VideoError(video_capture::VideoCaptureError),
    OpenCVError(opencv::Error),
    TOMLError(toml::de::Error),
}

impl From<video_capture::VideoCaptureError> for AppError {
    fn from(e: video_capture::VideoCaptureError) -> Self {
        AppError::VideoError(e)
    }
}

impl From<opencv::Error> for AppError {
    fn from(e: opencv::Error) -> Self {
        AppError::OpenCVError(e)
    }
}

impl From<toml::de::Error> for AppError {
    fn from(e: toml::de::Error) -> Self {
        AppError::TOMLError(e)
    }
}

pub struct App {
    pub input: InputSettings,
}

impl App {
    pub fn run(&mut self) -> Result<(), AppError> {
        let mut video_capture = video_capture::get_video_capture(self.input.video_source.as_str(), self.input.video_source_typ.clone())?;
        let (width, height, fps) = probe_video(&video_capture)?;
        println!("Video probe: {{Width: {width}px | Height: {height}px | FPS: {fps}}}");

        let opened = VideoCapture::is_opened(&video_capture).map_err(AppError::from)?;
        if !opened {
            return Err(AppError::Internal(AppInternalError{typ: 2}))
        }

        let mut empty_frames_countrer: u16 = 0;
        loop {
            let mut read_frame = Mat::default();
            match video_capture.read(&mut read_frame) {
                Ok(_) => {},
                Err(_) => {
                    println!("Can't read next frame");
                    break;
                }
            };
            if read_frame.empty() {
                println!("[WARNING]: Empty frame");
                empty_frames_countrer += 1;
                if empty_frames_countrer >= EMPTY_FRAMES_LIMIT {
                    println!("Too many empty frames");
                    break
                }
                continue;
            }
        }
        Ok(())
    }
}


fn probe_video(capture: &VideoCapture) ->  Result<(f32, f32, f32), AppError> {
    let fps = capture.get(opencv::videoio::CAP_PROP_FPS)? as f32;
    let frame_cols = capture.get(opencv::videoio::CAP_PROP_FRAME_WIDTH)? as f32;
    let frame_rows = capture.get(opencv::videoio::CAP_PROP_FRAME_HEIGHT)? as f32;
    // Is it better to get width/height from frame information?
    // let mut frame = Mat::default();
    // match capture.read(&mut frame) {
    //     Ok(_) => {},
    //     Err(_) => {
    //         return Err(AppError::VideoError(AppVideoError{typ: 2}));
    //     }
    // };
    // let frame_cols = frame.cols() as f32;
    // let frame_rows = frame.rows() as f32;
    Ok((frame_cols, frame_rows, fps))
}

#[derive(Serialize, Deserialize, Clone)]
pub struct InputSettings {
    #[serde(rename = "video_src")]
    pub video_source: String,
    #[serde(rename = "typ")]
    pub video_source_typ: String,
}

#[derive(Serialize, Deserialize)]
pub struct AppSettings {
    pub input: InputSettings,
}

impl AppSettings {
    pub fn new_from_file(filename: &str) -> Result<Self, AppError> {
        let toml_contents = fs::read_to_string(filename).expect(&format!("Something went wrong reading the file: '{}'", &filename));
        let app_settings = toml::from_str::<AppSettings>(&toml_contents)?;
        Ok(app_settings)
    }
    pub fn build(&self) -> Result<App, AppError> {
        Ok(App {
            input: self.input.clone(),
        })
    }
}

impl fmt::Display for AppSettings {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\tVideo input type: {}\n\tVideo URI: {}",
            self.input.video_source_typ,
            self.input.video_source,
        )
    }
}

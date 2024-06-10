use crate::video_capture;
use crate::app::app_settings;
use crate::app::app_error::AppError;
use crate::app::app_error::AppInternalError;

use opencv::{
    videoio::VideoCapture,
    prelude::VideoCaptureTrait,
    prelude::VideoCaptureTraitConst,
    prelude::MatTraitConst,
    core::Mat,
    core::Size,
    imgproc::resize,
    highgui::named_window,
    highgui::resize_window,
    highgui::imshow,
    highgui::wait_key,
};

const EMPTY_FRAMES_LIMIT: u16 = 60;

pub struct App {
    pub input: app_settings::InputSettings,
    pub output: app_settings::OutputSettings,
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
        let mut resized_frame = Mat::default();
        let window = &self.output.window_name;
        if self.output.enable {
            named_window(window, 1)?;
            resize_window(window, self.output.width, self.output.height)?;
        }

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

            if self.output.enable {
                resize(&read_frame, &mut resized_frame, Size::new(self.output.width, self.output.height), 1.0, 1.0, 1)?;
                if resized_frame.size()?.width > 0 {
                    imshow(window, &resized_frame)?;
                }
                let key = wait_key(10)?;
                if key == 27 /* esc */ || key == 115 /* s */ || key == 83 /* S */ {
                    break;
                }
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



use crate::video_capture;
use crate::video_capture::ThreadedFrame;

use crate::app::app_settings;
use crate::app::app_error::AppError;
use crate::app::app_error::AppInternalError;

use opencv::{
    bgsegm, core::Mat, core::{MatTraitConstManual, Size}, highgui::imshow, highgui::named_window, highgui::resize_window, highgui::wait_key, imgproc::resize, prelude::MatTraitConst, prelude::VideoCaptureTrait, prelude::VideoCaptureTraitConst, video::{create_background_subtractor_mog2, BackgroundSubtractorMOG2Trait, BackgroundSubtractorTrait, BackgroundSubtractorTraitConst}, videoio::VideoCapture
};

use std::{fs::read, thread};
use std::sync::mpsc;
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

        let (tx_capture, rx_capture): (mpsc::SyncSender<ThreadedFrame>, mpsc::Receiver<ThreadedFrame>) = mpsc::sync_channel(0);
        thread::spawn(move || {
            let mut frames_counter: f32 = 0.0;
            let mut total_seconds: f32 = 0.0;
            let mut overall_seconds: f32 = 0.0;
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
                frames_counter += 1.0;
                let second_fraction = total_seconds + (frames_counter / fps);
                if frames_counter >= fps {
                    total_seconds += 1.0;
                    overall_seconds += 1.0;
                    frames_counter = 0.0;
                }
                let frame = ThreadedFrame{
                    frame: read_frame,
                    overall_seconds: overall_seconds,
                    current_second: second_fraction,
                };
                match tx_capture.send(frame) {
                    Ok(_)=>{},
                    Err(_err) => {
                        // Closed channel?
                        // println!("Error on send frame to detection thread: {}", _err)
                    }
                };
            }
            match video_capture.release() {
                Ok(_) => {
                    println!("Video capture has been closed successfully");
                },
                Err(err) => {
                    println!("Can't release video capturer due the error: {}", err);
                }
            };
        });


        let mut resized_frame = Mat::default();
        let window = &self.output.window_name;
        if self.output.enable {
            named_window(window, 1)?;
            resize_window(window, self.output.width, self.output.height)?;
        }
       
        // let mut bg_subtractor = create_background_subtractor_mog2((1.0 * fps).floor() as i32, 16.0, false).unwrap();
        let mut bg_subtractor = opencv::bgsegm::create_background_subtractor_cnt(15, false, 15*60, true).unwrap();
        let mut foreground_mask = Mat::default();

        for received in rx_capture {
            let frame = received.frame.clone();
            // median_frame(vec![frame.clone()]);
            bg_subtractor.apply(&frame, &mut foreground_mask, -1.0).unwrap();
            let mut frame_background = Mat::default(); 
            bg_subtractor.get_background_image(&mut frame_background).unwrap();
            if self.output.enable {
                // resize(&frame_background, &mut resized_frame, Size::new(self.output.width, self.output.height), 1.0, 1.0, 1)?;
                resize(&frame, &mut resized_frame, Size::new(self.output.width, self.output.height), 1.0, 1.0, 1)?;
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

fn median_frame(frames: Vec<Mat>) {
    let rows = frames[0].rows();
    let cols = frames[0].cols();
    for frame in frames.iter() {
        // let mut b_channel = Mat::default();
        // let mut g_channel = Mat::default();
        // let mut r_channel = Mat::default();
        // opencv::core::extract_channel(&frame, &mut b_channel, 0);
        // opencv::core::extract_channel(&frame, &mut g_channel, 1);
        // opencv::core::extract_channel(&frame, &mut r_channel, 2);
    }
}

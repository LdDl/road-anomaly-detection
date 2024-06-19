use crate::video_capture;
use crate::video_capture::ThreadedFrame;

use crate::detection::process_yolo_detections;
use crate::tracker::Tracker;

use crate::app::app_settings;
use crate::app::app_error::AppError;
use crate::app::app_error::AppInternalError;

use opencv::{
    core::Mat, core::{Size, Rect, Scalar, get_cuda_enabled_device_count}, highgui::imshow, highgui::named_window, highgui::resize_window, highgui::wait_key, imgproc::resize, prelude::MatTraitConst, prelude::VideoCaptureTrait, prelude::VideoCaptureTraitConst, video::{create_background_subtractor_mog2, BackgroundSubtractorMOG2Trait, BackgroundSubtractorTraitConst}, videoio::VideoCapture,
    dnn::DNN_BACKEND_CUDA,
    dnn::DNN_TARGET_CUDA,
    dnn::DNN_BACKEND_OPENCV,
    dnn::DNN_TARGET_CPU,
    imgproc::LINE_4,
    imgproc::rectangle,
};

use std::{thread};
use std::sync::mpsc;
use std::collections::HashSet;
const EMPTY_FRAMES_LIMIT: u16 = 60;

use od_opencv::{
    model_format::ModelFormat,
    model_format::ModelVersion,
    model::new_from_file,
    model::ModelTrait,
};

pub struct App {
    pub input: app_settings::InputSettings,
    pub output: app_settings::OutputSettings,
    pub detection: app_settings::DetectionSettings,
    pub tracking: app_settings::TrackingSettings,
    pub model_format: ModelFormat,
    pub model_version: ModelVersion
}

impl App {
    pub fn run(&mut self) -> Result<(), AppError> {
        let mut neural_net = prepare_neural_net(self.model_format, self.model_version, &self.detection.network_weights, self.detection.network_cfg.clone(), (self.detection.net_width, self.detection.net_height))?;

        let mut video_capture = video_capture::get_video_capture(self.input.video_source.as_str(), self.input.video_source_typ.clone())?;
        let (width, height, fps) = probe_video(&video_capture)?;
        println!("Video probe: {{Width: {width}px | Height: {height}px | FPS: {fps}}}");

        let opened = VideoCapture::is_opened(&video_capture).map_err(AppError::from)?;
        if !opened {
            return Err(AppError::Internal(AppInternalError{typ: 2, txt: self.input.video_source.clone()}))
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
       
        let color_anomaly_bbox = Scalar::from((0.0, 0.0, 255.0));

        let mut bg_subtractor = create_background_subtractor_mog2((1.0 * fps).floor() as i32, 16.0, false).unwrap();
        // let mut bg_subtractor = opencv::bgsegm::create_background_subtractor_cnt(15, false, 15*60, true).unwrap();
        let mut foreground_mask = Mat::default();

        let conf_threshold: f32 = self.detection.conf_threshold;
        let nms_threshold: f32 = self.detection.nms_threshold;
        let target_classes = HashSet::from_iter(self.detection.target_classes.to_owned().unwrap_or(vec![]));
        let net_classes = self.detection.net_classes.to_owned();
        let time_frac = 1.0/fps;

        let mut tracker: Tracker = Tracker::new(fps.floor() as usize * self.tracking.delay_seconds, 0.3);
        println!("Tracker initialized with following settings:\n\t{}", tracker);
        for received in rx_capture {
            let mut frame = received.frame.clone();
            bg_subtractor.apply(&frame, &mut foreground_mask, -1.0).unwrap();
            let mut frame_background = Mat::default(); 
            bg_subtractor.get_background_image(&mut frame_background).unwrap();
            let (nms_bboxes, nms_classes_ids, nms_confidences) = match neural_net.forward(&frame_background, conf_threshold, nms_threshold) {
                Ok((a, b, c)) => { (a, b, c) },
                Err(err) => {
                    println!("Can't process input of neural network due the error {:?}", err);
                    break;
                }
            };
            let mut tmp_detections = process_yolo_detections(&nms_bboxes, nms_classes_ids, nms_confidences, &net_classes, &target_classes, time_frac);
            let relative_time = received.overall_seconds;
            tracker.match_objects(&mut tmp_detections, relative_time).unwrap();
            if self.output.enable {
                for object in tmp_detections.blobs.iter() {
                    let bbox = object.get_bbox();
                    let cv_rect = Rect::new(bbox.x.floor() as i32, bbox.y.floor() as i32, bbox.width as i32, bbox.height as i32);
                    match rectangle(&mut frame, cv_rect, color_anomaly_bbox, 2, LINE_4, 0) {
                        Ok(_) => {},
                        Err(err) => {
                            panic!("Can't draw rectangle at blob's bbox due the error: {:?}", err)
                        }
                    };
                }
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

fn prepare_neural_net(mf: ModelFormat, mv: ModelVersion, weights: &str, configuration: Option<String>, net_size: (i32, i32)) -> Result<Box<dyn ModelTrait>, AppError> {

    /* Check if CUDA is an option at all */
    let cuda_count = get_cuda_enabled_device_count()?;
    let cuda_available = cuda_count > 0;
    println!("CUDA is {}", if cuda_available { "'available'" } else { "'not available'" });
    println!("Model format is '{:?}'", mf);
    println!("Model type is '{:?}'", mv);

    // Hacky way to convert Option<String> to Option<&str>
    let configuration_str = configuration.as_deref();

    let neural_net = match new_from_file(
        weights,
        configuration_str,
        (net_size.0, net_size.1),
        mf, mv,
        if cuda_available { DNN_BACKEND_CUDA } else { DNN_BACKEND_OPENCV },
        if cuda_available { DNN_TARGET_CUDA } else { DNN_TARGET_CPU },
        vec![]
    ) {
        Ok(result) => result,
        Err(err) => {
            panic!("Can't read network '{}' (with cfg '{:?}') due the error: {:?}", weights, configuration, err);
        }
    };
    Ok(neural_net)
}


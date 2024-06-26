use crate::{publisher::PublisherTrait, video_capture};
use crate::video_capture::ThreadedFrame;

use crate::detection::process_yolo_detections;
use crate::tracker::Tracker;
use crate::zones::Zone;
use crate::events::EventInfo;
use crate::publisher::redis_publisher::RedisConnection;
use crate::draw::{invert_color, draw_bboxes, draw_identifiers};

use crate::app::app_settings;
use crate::app::app_error::AppError;
use crate::app::app_error::AppInternalError;

use opencv::{
    core::Mat, core::{Size, Scalar, get_cuda_enabled_device_count}, highgui::imshow, highgui::named_window, highgui::resize_window, highgui::wait_key, imgproc::resize, prelude::MatTraitConst, prelude::VideoCaptureTrait, prelude::VideoCaptureTraitConst, video::{create_background_subtractor_mog2, BackgroundSubtractorMOG2Trait, BackgroundSubtractorTraitConst}, videoio::VideoCapture,
    dnn::DNN_BACKEND_CUDA,
    dnn::DNN_TARGET_CUDA,
    dnn::DNN_BACKEND_OPENCV,
    dnn::DNN_TARGET_CPU,
};

use std::thread;
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
    pub application_info: app_settings::ApplicationInfo,
    pub input: app_settings::InputSettings,
    pub output: app_settings::OutputSettings,
    pub detection: app_settings::DetectionSettings,
    pub tracking: app_settings::TrackingSettings,
    pub zones_settings: Option<Vec<app_settings::ZoneSettings>>,
    pub publishers: Option<app_settings::PublishersSettings>,
    pub model_format: ModelFormat,
    pub model_version: ModelVersion,
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
                    overall_seconds,
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
                    eprintln!("Can't release video capturer due the error: {}", err);
                }
            };
        });


        let mut resized_frame = Mat::default();
        let window = &self.output.window_name;
        if self.output.enable {
            named_window(window, 1)?;
            resize_window(window, self.output.width, self.output.height)?;
        }
       
        let bbox_scalar: Scalar = Scalar::from((0.0, 0.0, 255.0));
        let bbox_scalar_inverse:Scalar = invert_color(&bbox_scalar);
        let id_scalar: Scalar = Scalar::from((0.0, 0.0, 255.0));
        let id_scalar_inverse: Scalar = invert_color(&id_scalar);

        let mut bg_subtractor = create_background_subtractor_mog2((1.0 * fps).floor() as i32, 16.0, false)?;
        // let mut bg_subtractor = opencv::bgsegm::create_background_subtractor_cnt(15, false, 15*60, true)?;
        let mut foreground_mask = Mat::default();

        let conf_threshold: f32 = self.detection.conf_threshold;
        let nms_threshold: f32 = self.detection.nms_threshold;
        let target_classes = HashSet::from_iter(self.detection.target_classes.to_owned().unwrap_or(vec![]));
        let net_classes = self.detection.net_classes.to_owned();
        let time_frac = 1.0/fps;
        
        let lifetime_seconds_min = self.tracking.lifetime_seconds_min as i64;
        let lifetime_seconds_max = self.tracking.lifetime_seconds_max as i64;
        let mut tracker: Tracker = Tracker::new(fps.floor() as usize, 0.3);
        println!("Tracker initialized with following settings:\n\t{}", tracker);
   
        let app_name = self.application_info.id.to_owned();

        let mut zones: Vec<Zone> = match self.zones_settings.clone() {
            Some(d) => {
                d.iter().map(|zone_settings| {
                    Zone::new(zone_settings.id.clone(), zone_settings.geometry, zone_settings.color_rgb)
                }).collect()
            }
            None => vec![Zone::new("whole_image".to_string(), [[5, 5], [width as i32 - 5, 5], [width as i32 - 5, height as i32 - 5], [5, height as i32 - 5]], Some([0, 0, 255]))]
        };
        
        // Init publishers
        let (events_sender, events_reciever): (mpsc::SyncSender<EventInfo>, mpsc::Receiver<EventInfo>) = mpsc::sync_channel(0);
        let publishers_settings = self.publishers.to_owned();
        thread::spawn(move || {
            let mut publishers: Vec<Box<dyn PublisherTrait>> = vec![];
            match publishers_settings {
                Some(ps) => {
                    match ps.redis {
                        Some(redis_settings) => {
                            let redis_conn = if redis_settings.password.is_empty() {
                                RedisConnection::new(redis_settings.host, redis_settings.port, redis_settings.db_index, redis_settings.channel_name)
                            } else {
                                if redis_settings.username.is_empty() {
                                    RedisConnection::new_with_password(redis_settings.host, redis_settings.port, redis_settings.db_index, redis_settings.channel_name, redis_settings.password)
                                } else {
                                    RedisConnection::new_with_username_password(redis_settings.host, redis_settings.port, redis_settings.db_index, redis_settings.channel_name, redis_settings.username, redis_settings.password)
                                }
                            };
                            match redis_conn {
                                Ok(conn) => publishers.push(conn),
                                Err(e) => eprintln!("Failed to create Redis connection: {}. Ignoring Redis publisher", e),
                            }
                        },
                        None => {}
                    }
                },
                None => {}
            }
            events_processing(events_reciever, publishers);
        });

        let mut resized_frame_for_bg = Mat::default();
        let scale_width = width / self.detection.net_width as f32;
        let scale_height = height / self.detection.net_height as f32;

        for received in rx_capture {
            let mut frame = received.frame.clone();
            // We need to resize image despite of neural network class (DNN module resizes image) since we need to speed up background subtractor
            resize(&frame, &mut resized_frame_for_bg, Size::new(self.detection.net_width, self.detection.net_height), 1.0, 1.0, 1)?;
            bg_subtractor.apply(&resized_frame_for_bg, &mut foreground_mask, -1.0)?;
            let mut frame_background = Mat::default(); 
            bg_subtractor.get_background_image(&mut frame_background)?;
            let (nms_bboxes, nms_classes_ids, nms_confidences) = match neural_net.forward(&frame_background, conf_threshold, nms_threshold) {
                Ok((a, b, c)) => { (a, b, c) },
                Err(err) => {
                    eprintln!("Can't process input of neural network due the error {:?}", err);
                    break;
                }
            };
            let mut tmp_detections = process_yolo_detections(&nms_bboxes, nms_classes_ids, nms_confidences, &net_classes, &target_classes, time_frac, scale_width, scale_height);
            let relative_time = received.overall_seconds;
            tracker.match_objects(&mut tmp_detections, relative_time).unwrap();
            
            for zone in zones.iter_mut() {
                let registered_events = zone.process_tracker(&mut tracker, lifetime_seconds_min, lifetime_seconds_max, Some(app_name.clone()), Some(&frame))?;
                for new_event in registered_events {
                    match events_sender.send(new_event) {
                        Ok(_)=>{ },
                        Err(_err) => {
                            // Closed channel?
                            eprintln!("Error on send event to postprocess thread: {}", _err)
                        }
                    };
                }
            }
            if self.output.enable {
                draw_bboxes(&mut frame, &tracker, bbox_scalar, bbox_scalar_inverse);
                draw_identifiers(&mut frame, &tracker, id_scalar, id_scalar_inverse);
                for zone in zones.iter() {
                    zone.draw(&mut frame)?;
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

    let neural_net = new_from_file(
        weights,
        configuration_str,
        (net_size.0, net_size.1),
        mf, mv,
        if cuda_available { DNN_BACKEND_CUDA } else { DNN_BACKEND_OPENCV },
        if cuda_available { DNN_TARGET_CUDA } else { DNN_TARGET_CPU },
        vec![]
    )?;
    Ok(neural_net)
}

fn events_processing(events_reciever: mpsc::Receiver<EventInfo>, publishers: Vec<Box<dyn PublisherTrait>>) {
    for event_income in events_reciever {
        for publisher in publishers.iter() {
            match publisher.publish(&event_income) {
                Ok(_) => {},
                Err(err) => {
                    eprintln!("Error during publishing message: {:#?}", err);
                }
            };
        }
    }
}

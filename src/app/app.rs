use crate::{publisher::PublisherTrait, video_capture};
use crate::video_capture::ThreadedFrame;

use crate::detection::process_yolo_detections;
use crate::tracker::Tracker;
use crate::zones::Zone;
use crate::events::EventInfo;
use crate::publisher::redis_publisher::RedisConnection;
use crate::draw::{Scalar, invert_color, draw_bboxes, draw_identifiers};
use crate::background::BackgroundSubtractorMOG2;

use crate::app::app_settings;
use crate::app::app_error::AppError;
use minifb::{Key, ScaleMode, Window, WindowOptions};

use std::thread;
use std::sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}};
use std::collections::HashSet;

use od_opencv::{Model, ModelUltralyticsOrt};

pub struct App {
    pub application_info: app_settings::ApplicationInfo,
    pub input: app_settings::InputSettings,
    pub output: app_settings::OutputSettings,
    pub detection: app_settings::DetectionSettings,
    pub tracking: app_settings::TrackingSettings,
    pub zones_settings: Option<Vec<app_settings::ZoneSettings>>,
    pub publishers: Option<app_settings::PublishersSettings>,
}

impl App {
    pub fn run(&mut self) -> Result<(), AppError> {
        let mut neural_net = prepare_neural_net(&self.detection.network_weights, (self.detection.net_width, self.detection.net_height))?;

        let mut video_capture = video_capture::get_video_capture(self.input.video_source.as_str(), self.input.video_source_typ.clone())?;
        let (width, height, fps) = (video_capture.width as f32, video_capture.height as f32, video_capture.fps);
        println!("Video probe: {{Width: {width}px | Height: {height}px | FPS: {fps}}}");

        let capture_process = video_capture.process();
        let signal_capture = video_capture.process();
        let running = Arc::new(AtomicBool::new(true));
        let signal_running = running.clone();
        ctrlc::set_handler(move || {
            signal_running.store(false, Ordering::Relaxed);
            signal_capture.stop();
        })?;

        let (tx_capture, rx_capture): (mpsc::SyncSender<ThreadedFrame>, mpsc::Receiver<ThreadedFrame>) = mpsc::sync_channel(0);
        thread::spawn(move || {
            let mut frames_counter: f32 = 0.0;
            let mut total_seconds: f32 = 0.0;
            let mut overall_seconds: f32 = 0.0;
            loop {
                let read_frame = match video_capture.read_frame() {
                    Ok(Some(frame)) => frame,
                    Ok(None) => break,
                    Err(err) => {
                        eprintln!("Can't read next frame: {}", err);
                        break;
                    }
                };
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
                        break;
                    }
                };
            }
            println!("Video capture has been closed successfully");
        });


        let mut window = if self.output.enable {
            Some(Window::new(&self.output.window_name, self.output.width as usize, self.output.height as usize, WindowOptions{resize: true, scale_mode: ScaleMode::Stretch, ..WindowOptions::default()})?)
        } else {
            None
        };
       
        let bbox_scalar: Scalar = Scalar::from((0, 0, 255));
        let bbox_scalar_inverse:Scalar = invert_color(&bbox_scalar);
        let id_scalar: Scalar = Scalar::from((0, 0, 255));
        let id_scalar_inverse: Scalar = invert_color(&id_scalar);

        let mut bg_subtractor = BackgroundSubtractorMOG2::new(fps.floor() as usize, 16.0);

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

        let scale_width = width / self.detection.net_width as f32;
        let scale_height = height / self.detection.net_height as f32;

        for received in rx_capture {
            if !running.load(Ordering::Relaxed) {
                break;
            }
            let mut frame = received.frame;
            // Run MOG2 at the network resolution before passing its background to YOLO.
            let resized_frame_for_bg = frame.resize(self.detection.net_width as u32, self.detection.net_height as u32);
            let frame_background = bg_subtractor.apply(&resized_frame_for_bg);
            let (nms_bboxes, nms_classes_ids, nms_confidences) = match neural_net.forward(&frame_background.to_image_buffer(), conf_threshold, nms_threshold) {
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
                let registered_events = zone.process_tracker(&mut tracker, lifetime_seconds_min, lifetime_seconds_max, Some(app_name.clone()), Some(&frame));
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
            if let Some(window) = window.as_mut() {
                draw_bboxes(&mut frame, &tracker, bbox_scalar, bbox_scalar_inverse);
                draw_identifiers(&mut frame, &tracker, id_scalar, id_scalar_inverse);
                for zone in zones.iter() {
                    zone.draw(&mut frame);
                }
                let resized_frame = frame.resize(self.output.width as u32, self.output.height as u32);
                window.update_with_buffer(&resized_frame.to_window_buffer(), resized_frame.width as usize, resized_frame.height as usize)?;
                if !window.is_open() || window.is_key_down(Key::Escape) || window.is_key_down(Key::S) {
                    break;
                }
            }
        }

        capture_process.stop();
        Ok(())
    }
}

fn prepare_neural_net(weights: &str, net_size: (i32, i32)) -> Result<ModelUltralyticsOrt, AppError> {
    let net_size = (net_size.0 as u32, net_size.1 as u32);
    #[cfg(feature = "ort-cuda")]
    report_cuda_provider();
    #[cfg(feature = "ort-cuda")]
    let neural_net = Model::ort_cuda(weights, net_size)?;
    #[cfg(not(feature = "ort-cuda"))]
    let neural_net = Model::ort(weights, net_size)?;
    println!("Model backend is ONNX Runtime");
    Ok(neural_net)
}

/// Registration of the CUDA execution provider is silent by default: ONNX Runtime falls back to
/// the CPU without any message. Register it once with `error_on_failure` to learn whether the
/// session built later actually runs on the GPU. Only the session options are created here, so no
/// model is loaded and the check is cheap.
#[cfg(feature = "ort-cuda")]
fn report_cuda_provider() {
    use ort::execution_providers::CUDAExecutionProvider;
    use ort::session::Session;

    let probe = match Session::builder() {
        Ok(builder) => builder
            .with_execution_providers([CUDAExecutionProvider::default().build().error_on_failure()])
            .map(|_| ())
            .map_err(|err| err.to_string()),
        Err(err) => Err(err.to_string()),
    };
    match probe {
        Ok(()) => println!("CUDA execution provider is registered: inference runs on GPU"),
        Err(err) => {
            eprintln!("CUDA execution provider is NOT available, inference falls back to CPU: {err}");
            eprintln!("Hint: libonnxruntime_providers_cuda.so and libonnxruntime_providers_shared.so must be placed next to the executable");
        }
    }
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

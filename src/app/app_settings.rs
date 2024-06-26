use crate::app::{app_error::AppError, AppInternalError};
use crate::app::App;
use serde::{ Deserialize, Serialize };
use std::fs;
use std::fmt;
use od_opencv::model_format::{ModelFormat, ModelVersion};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationInfo {
    pub id: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct InputSettings {
    #[serde(rename = "video_src")]
    pub video_source: String,
    #[serde(rename = "typ")]
    pub video_source_typ: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OutputSettings {
    pub enable: bool,
    pub width: i32,
    pub height: i32,
    pub window_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DetectionSettings {
    pub network_ver: Option<i32>,
    pub network_format: Option<String>,
    pub network_weights: String,
    pub network_cfg: Option<String>,
    pub conf_threshold: f32,
    pub nms_threshold: f32,
    pub net_width: i32,
    pub net_height: i32,
    pub net_classes: Vec<String>,
    pub target_classes: Option<Vec<String>>,
}

impl DetectionSettings {
    pub fn get_nn_format(&self) -> Result<ModelFormat, AppError> {
        match self.network_format.clone() {
            Some(mf) => {
                match mf.to_lowercase().as_str() {
                    "darknet" => { Ok(ModelFormat::Darknet) },
                    "onnx" => { Ok(ModelFormat::ONNX) },
                    _ => { 
                       Err(AppError::from(AppInternalError{typ: 3, txt: mf}))
                    }
                }
            },
            None => { Ok(ModelFormat::Darknet) }
        }
    }
    pub fn get_nn_version(&self) -> Result<ModelVersion, AppError> {
        match self.network_ver {
            Some(mv) => {
                match mv {
                    3 => { Ok(ModelVersion::V3) },
                    4 => { Ok(ModelVersion::V4) },
                    7 => { Ok(ModelVersion::V7) },
                    8 => { Ok(ModelVersion::V8) },
                    _ => { 
                        Err(AppError::from(AppInternalError{typ: 4, txt: format!("{}", mv)}))
                    }
                }
            },
            None => { Ok(ModelVersion::V3) }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrackingSettings {
    pub delay_seconds: usize,
    pub lifetime_seconds_min: u64,
    pub lifetime_seconds_max: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZoneSettings {
    pub id: String,
    pub geometry: [[i32; 2]; 4],
    pub color_rgb: Option<[u16; 3]>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PublishersSettings {
    pub redis: Option<RedisPublisherSettings>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RedisPublisherSettings {
    pub enable: bool,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub password: String,
    pub db_index: i32,
    pub channel_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct AppSettings {
    pub application_info: ApplicationInfo,
    pub input: InputSettings,
    pub output: OutputSettings,
    pub detection: DetectionSettings,
    pub tracking: TrackingSettings,
    pub zones: Option<Vec<ZoneSettings>>,
    pub publishers: Option<PublishersSettings>,
}

impl AppSettings {
    pub fn new_from_file(filename: &str) -> Result<Self, AppError> {
        let toml_contents = fs::read_to_string(filename).expect(&format!("Something went wrong reading the file: '{}'", &filename));
        let app_settings = toml::from_str::<AppSettings>(&toml_contents)?;
        Ok(app_settings)
    }

    pub fn build(&self) -> Result<App, AppError> {
        let mf = self.detection.get_nn_format()?;
        let mv = self.detection.get_nn_version()?;
        if self.tracking.lifetime_seconds_min >= self.tracking.lifetime_seconds_max {
            return Err(AppError::from(AppInternalError{typ: 5, txt: format!("Incorrect lifetimes. Min: {}, Max: {}", self.tracking.lifetime_seconds_min, self.tracking.lifetime_seconds_max)}));
        }
        Ok(App {
            application_info: self.application_info.clone(),
            input: self.input.clone(),
            output: self.output.clone(),
            detection: self.detection.clone(),
            tracking: self.tracking.clone(),
            zones_settings: self.zones.clone(),
            publishers: self.publishers.clone(),
            model_format: mf,
            model_version: mv
        })
    }
}

impl fmt::Display for AppSettings {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\tApplication name: {}\n\tVideo input type: {}\n\tVideo URI: {}\n\tNetwork type: {:?}\n\tNetwork version: {:?}\n\tNetwork weights: {}\n\tNetwork configuration: {:?}\n\tTracker delay seconds: {}\n\tTracker min lifetime seconds: {}",
            self.application_info.id,
            self.input.video_source_typ,
            self.input.video_source,
            self.detection.network_format,
            self.detection.network_ver,
            self.detection.network_weights,
            self.detection.network_cfg,
            self.tracking.delay_seconds,
            self.tracking.lifetime_seconds_min
        )
    }
}

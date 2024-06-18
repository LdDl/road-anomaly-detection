use crate::app::app_error::AppError;
use crate::app::App;
use serde::{ Deserialize, Serialize };
use std::fs;
use std::fmt;
use std::error::Error;
use od_opencv::model_format::{ModelFormat, ModelVersion};

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
    pub fn get_nn_format(&self) -> Result<ModelFormat,  Box<dyn Error>> {
        match self.network_format.clone() {
            Some(mf) => {
                match mf.to_lowercase().as_str() {
                    "darknet" => { Ok(ModelFormat::Darknet) },
                    "onnx" => { Ok(ModelFormat::ONNX) },
                    _ => { 
                        return Err(format!("Can't prepare neural network due the unhandled format: {}", mf).into());
                    }
                }
            },
            None => { Ok(ModelFormat::Darknet) }
        }
    }
    pub fn get_nn_version(&self) -> Result<ModelVersion,  Box<dyn Error>> {
        match self.network_ver.clone() {
            Some(mv) => {
                match mv {
                    3 => { Ok(ModelVersion::V3) },
                    4 => { Ok(ModelVersion::V4) },
                    7 => { Ok(ModelVersion::V7) },
                    8 => { Ok(ModelVersion::V8) },
                    _ => { 
                        return Err(format!("Can't prepare neural network due the unhandled version: {}", mv).into());
                    }
                }
            },
            None => { Ok(ModelVersion::V3) }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct AppSettings {
    pub input: InputSettings,
    pub output: OutputSettings,
    pub detection: DetectionSettings,
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
            output: self.output.clone(),
            detection: self.detection.clone()
        })
    }
}

impl fmt::Display for AppSettings {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\tVideo input type: {}\n\tVideo URI: {}\n\tNetwork type: {:?}\n\tNetwork version: {:?}\n\tNetwork weights: {}\n\tNetwork configuration: {:?}",
            self.input.video_source_typ,
            self.input.video_source,
            self.detection.network_format,
            self.detection.network_ver,
            self.detection.network_weights,
            self.detection.network_cfg,
        )
    }
}

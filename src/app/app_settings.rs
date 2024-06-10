use crate::app::app_error::AppError;
use crate::app::App;
use serde::{ Deserialize, Serialize };
use std::fs;
use std::fmt;

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

#[derive(Serialize, Deserialize)]
pub struct AppSettings {
    pub input: InputSettings,
    pub output: OutputSettings,
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
            output: self.output.clone()
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
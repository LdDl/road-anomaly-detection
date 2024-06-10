use std::env;
use road_anomaly_detection::app::{
    AppSettings,
    AppError
};

fn main() -> Result<(), AppError> {
    let args: Vec<String> = env::args().collect();
    let path_to_config = match args.len() {
        2 => {
            &args[1]
        },
        _ => {
            println!("Args should contain exactly one string: path to TOML configuration file. Setting to default './data/conf.toml'");
            "./data/conf.toml"
        }
    };
    let app_settings = AppSettings::new_from_file(path_to_config)?;
    println!("Settings are:\n{}", app_settings);
    let mut app = app_settings.build()?;
    app.run()?;

    Ok(())
}

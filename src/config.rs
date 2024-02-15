use crate::models::Config;
use std::fs;

pub fn read_config(config_path: &str) -> Config {
    let config_content = fs::read_to_string(config_path).expect("Failed to read config file.");
    let config: Config = toml::from_str(&config_content).expect("Failed to parse config file");
    println!("Loaded repo configurations");
    config
}

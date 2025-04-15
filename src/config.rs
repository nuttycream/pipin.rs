use std::{
    fs,
    io,
};

use crate::actions::Action;
use serde::{
    Deserialize,
    Serialize,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub device: i32,
    pub actions: Vec<Action>,
}

const DEFAULT_CONF_NAME: &str = "config";

pub fn save_conf(config: &Config) -> io::Result<()> {
    let file_name = format!("{}.json", DEFAULT_CONF_NAME);
    let jason = serde_json::to_string_pretty(config)?;
    fs::write(file_name, jason)?;
    Ok(())
}

pub fn load_conf() -> io::Result<Config> {
    let file_name = format!("{}.json", DEFAULT_CONF_NAME);
    let json = fs::read_to_string(file_name)?;
    let config = serde_json::from_str(&json)?;
    Ok(config)
}

pub fn save_actions(actions: &[Action]) -> io::Result<()> {
    let device = match load_conf() {
        Ok(conf) => conf.device,
        Err(_) => 0,
    };

    let conf = Config {
        device,
        actions: actions.to_vec(),
    };

    save_conf(&conf)
}

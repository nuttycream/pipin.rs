use crate::actions::Action;

use serde::{Deserialize, Serialize};
use std::{fs, io};

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
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
    match fs::read_to_string(file_name) {
        Ok(json) => {
            let config = serde_json::from_str(&json)?;
            Ok(config)
        }
        Err(e) => {
            if e.kind() == io::ErrorKind::NotFound {
                let default_config = Config {
                    actions: Vec::new(),
                };
                save_conf(&default_config)?;
                Ok(default_config)
            } else {
                Err(e)
            }
        }
    }
}

pub fn save_actions(actions: &[Action]) -> io::Result<()> {
    let mut config = match load_conf() {
        Ok(conf) => conf,
        Err(_) => Config {
            actions: Vec::new(),
        },
    };

    config.actions = actions.to_vec();
    save_conf(&config)
}

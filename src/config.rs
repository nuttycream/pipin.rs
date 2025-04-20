use std::{fs, io};

use crate::actions::Action;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct DataPin {
    //gio power gnd
    pub pin_type: String,
    pub pin: Option<String>,
    pub label: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PinRow {
    pub left: DataPin,
    pub right: DataPin,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct GpioPins {
    pub rows: Vec<PinRow>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub device: i32,
    pub actions: Vec<Action>,
    pub gpio_pins: GpioPins,
}

// is this ok in rust
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
                    device: 0,
                    actions: Vec::new(),
                    gpio_pins: default_pins(),
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
            device: 0,
            actions: Vec::new(),
            gpio_pins: default_pins(),
        },
    };

    config.actions = actions.to_vec();
    save_conf(&config)
}

pub fn default_pins() -> GpioPins {
    GpioPins {
        rows: vec![
            PinRow {
                left: DataPin {
                    pin_type: "power".to_string(),
                    pin: None,
                    label: "3.3V".to_string(),
                },
                right: DataPin {
                    pin_type: "power".to_string(),
                    pin: None,
                    label: "5V".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("2".to_string()),
                    label: "GPIO 2".to_string(),
                },
                right: DataPin {
                    pin_type: "power".to_string(),
                    pin: None,
                    label: "5V".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("3".to_string()),
                    label: "GPIO 3".to_string(),
                },
                right: DataPin {
                    pin_type: "ground".to_string(),
                    pin: None,
                    label: "GND".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("4".to_string()),
                    label: "GPIO 4".to_string(),
                },
                right: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("14".to_string()),
                    label: "GPIO 14".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: "ground".to_string(),
                    pin: None,
                    label: "GND".to_string(),
                },
                right: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("15".to_string()),
                    label: "GPIO 15".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("17".to_string()),
                    label: "GPIO 17".to_string(),
                },
                right: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("18".to_string()),
                    label: "GPIO 18".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("27".to_string()),
                    label: "GPIO 27".to_string(),
                },
                right: DataPin {
                    pin_type: "ground".to_string(),
                    pin: None,
                    label: "GND".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("22".to_string()),
                    label: "GPIO 22".to_string(),
                },
                right: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("23".to_string()),
                    label: "GPIO 23".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: "power".to_string(),
                    pin: None,
                    label: "3.3V".to_string(),
                },
                right: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("24".to_string()),
                    label: "GPIO 24".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("10".to_string()),
                    label: "GPIO 10".to_string(),
                },
                right: DataPin {
                    pin_type: "ground".to_string(),
                    pin: None,
                    label: "GND".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("9".to_string()),
                    label: "GPIO 9".to_string(),
                },
                right: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("25".to_string()),
                    label: "GPIO 25".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("11".to_string()),
                    label: "GPIO 11".to_string(),
                },
                right: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("8".to_string()),
                    label: "GPIO 8".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: "ground".to_string(),
                    pin: None,
                    label: "GND".to_string(),
                },
                right: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("7".to_string()),
                    label: "GPIO 7".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("0".to_string()),
                    label: "GPIO 0".to_string(),
                },
                right: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("1".to_string()),
                    label: "GPIO 1".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("5".to_string()),
                    label: "GPIO 5".to_string(),
                },
                right: DataPin {
                    pin_type: "ground".to_string(),
                    pin: None,
                    label: "GND".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("6".to_string()),
                    label: "GPIO 6".to_string(),
                },
                right: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("12".to_string()),
                    label: "GPIO 12".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("13".to_string()),
                    label: "GPIO 13".to_string(),
                },
                right: DataPin {
                    pin_type: "ground".to_string(),
                    pin: None,
                    label: "GND".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("19".to_string()),
                    label: "GPIO 19".to_string(),
                },
                right: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("16".to_string()),
                    label: "GPIO 16".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("26".to_string()),
                    label: "GPIO 26".to_string(),
                },
                right: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("20".to_string()),
                    label: "GPIO 20".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: "ground".to_string(),
                    pin: None,
                    label: "GND".to_string(),
                },
                right: DataPin {
                    pin_type: "gpio".to_string(),
                    pin: Some("21".to_string()),
                    label: "GPIO 21".to_string(),
                },
            },
        ],
    }
}

pub fn create_pin_html(pin: &DataPin) -> String {
    let mut html = String::new();

    html.push_str(&format!("<button class=\"pin {}\"", pin.pin_type));

    if pin.pin_type != "gpio" {
        html.push_str(" disabled");
    } else if let Some(pin_num) = &pin.pin {
        html.push_str(&format!(
            " data-pin=\"{}\" ws-send 
                hx-trigger=\"click\" 
                hx-vals='{{\"pin\": \"{}\"}}'",
            pin_num, pin_num
        ));
    }

    html.push_str(&format!(">{}</button>", pin.label));

    html
}

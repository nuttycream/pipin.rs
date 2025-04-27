use crate::actions::Action;
use crate::gpio::PinType;

use serde::{Deserialize, Serialize};
use std::{fs, io};

#[derive(Serialize, Deserialize, Clone)]
pub struct DataPin {
    pub pin_type: PinType,
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

// todo refactor
pub fn create_pin_html(pin: &DataPin) -> String {
    let mut html = String::new();

    let powered = match pin.pin_type {
        PinType::Power5v | PinType::Power3v3 => "power",
        PinType::Gnd => "ground",
        _ => "gpio",
    };

    // handle the different gpio types
    // thinking this should be another enum
    // like GpioType { i2c, spi, etc }
    let is_gpio = matches!(
        pin.pin_type,
        PinType::Gpio
            | PinType::I2c
            | PinType::Spi
            | PinType::Uart
            | PinType::Pcm
    );

    if is_gpio && pin.pin.is_some() {
        // going to unwrap for pin # since
        // i doubt we'll never have it.
        let pin_num = pin.pin.as_ref().unwrap();
        html.push_str(&format!(
            "<button id={} class=\"pin {}\"",
            pin_num, powered
        ));
    } else {
        html.push_str(&format!("<button class=\"pin {}\" disabled", powered));
    }

    if let Some(pin_num) = &pin.pin {
        html.push_str(&format!(
            " ws-send hx-trigger=\"click\" hx-vals='{{\"pin\": \"{}\"}}'",
            pin_num
        ));
    }

    html.push_str(&format!(">{}</button>", pin.label));

    html
}

// todo refactor
pub fn default_pins() -> GpioPins {
    GpioPins {
        rows: vec![
            PinRow {
                left: DataPin {
                    pin_type: PinType::Power3v3,
                    pin: None,
                    label: "3v3 Power".to_string(),
                },
                right: DataPin {
                    pin_type: PinType::Power5v,
                    pin: None,
                    label: "5v Power".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: PinType::I2c,
                    pin: Some("2".to_string()),
                    label: "GPIO 2 (I2C1 SDA)".to_string(),
                },
                right: DataPin {
                    pin_type: PinType::Power5v,
                    pin: None,
                    label: "5v Power".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: PinType::I2c,
                    pin: Some("3".to_string()),
                    label: "GPIO 3 (I2C1 SCL)".to_string(),
                },
                right: DataPin {
                    pin_type: PinType::Gnd,
                    pin: None,
                    label: "Ground".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: PinType::Gpio,
                    pin: Some("4".to_string()),
                    label: "GPIO 4 (GPCLK0)".to_string(),
                },
                right: DataPin {
                    pin_type: PinType::Uart,
                    pin: Some("14".to_string()),
                    label: "GPIO 14 (UART TX)".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: PinType::Gnd,
                    pin: None,
                    label: "Ground".to_string(),
                },
                right: DataPin {
                    pin_type: PinType::Uart,
                    pin: Some("15".to_string()),
                    label: "GPIO 15 (UART RX)".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: PinType::Gpio,
                    pin: Some("17".to_string()),
                    label: "GPIO 17".to_string(),
                },
                right: DataPin {
                    pin_type: PinType::Pcm,
                    pin: Some("18".to_string()),
                    label: "GPIO 18 (PCM CLK)".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: PinType::Gpio,
                    pin: Some("27".to_string()),
                    label: "GPIO 27".to_string(),
                },
                right: DataPin {
                    pin_type: PinType::Gnd,
                    pin: None,
                    label: "Ground".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: PinType::Gpio,
                    pin: Some("22".to_string()),
                    label: "GPIO 22".to_string(),
                },
                right: DataPin {
                    pin_type: PinType::Gpio,
                    pin: Some("23".to_string()),
                    label: "GPIO 23".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: PinType::Power3v3,
                    pin: None,
                    label: "3v3 Power".to_string(),
                },
                right: DataPin {
                    pin_type: PinType::Gpio,
                    pin: Some("24".to_string()),
                    label: "GPIO 24".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: PinType::Spi,
                    pin: Some("10".to_string()),
                    label: "GPIO 10 (SPI0 MOSI)".to_string(),
                },
                right: DataPin {
                    pin_type: PinType::Gnd,
                    pin: None,
                    label: "Ground".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: PinType::Spi,
                    pin: Some("9".to_string()),
                    label: "GPIO 9 (SPI0 MISO)".to_string(),
                },
                right: DataPin {
                    pin_type: PinType::Gpio,
                    pin: Some("25".to_string()),
                    label: "GPIO 25".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: PinType::Spi,
                    pin: Some("11".to_string()),
                    label: "GPIO 11 (SPI0 SCLK)".to_string(),
                },
                right: DataPin {
                    pin_type: PinType::Spi,
                    pin: Some("8".to_string()),
                    label: "GPIO 8 (SPI0 CE0)".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: PinType::Gnd,
                    pin: None,
                    label: "Ground".to_string(),
                },
                right: DataPin {
                    pin_type: PinType::Spi,
                    pin: Some("7".to_string()),
                    label: "GPIO 7 (SPI0 CE1)".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: PinType::I2c,
                    pin: Some("0".to_string()),
                    label: "GPIO 0 (EEPROM SDA)".to_string(),
                },
                right: DataPin {
                    pin_type: PinType::I2c,
                    pin: Some("1".to_string()),
                    label: "GPIO 1 (EEPROM SCL)".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: PinType::Gpio,
                    pin: Some("5".to_string()),
                    label: "GPIO 5".to_string(),
                },
                right: DataPin {
                    pin_type: PinType::Gnd,
                    pin: None,
                    label: "Ground".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: PinType::Gpio,
                    pin: Some("6".to_string()),
                    label: "GPIO 6".to_string(),
                },
                right: DataPin {
                    pin_type: PinType::Gpio,
                    pin: Some("12".to_string()),
                    label: "GPIO 12 (PWM0)".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: PinType::Gpio,
                    pin: Some("13".to_string()),
                    label: "GPIO 13 (PWM1)".to_string(),
                },
                right: DataPin {
                    pin_type: PinType::Gnd,
                    pin: None,
                    label: "Ground".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: PinType::Pcm,
                    pin: Some("19".to_string()),
                    label: "GPIO 19 (PCM FS)".to_string(),
                },
                right: DataPin {
                    pin_type: PinType::Gpio,
                    pin: Some("16".to_string()),
                    label: "GPIO 16".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: PinType::Gpio,
                    pin: Some("26".to_string()),
                    label: "GPIO 26".to_string(),
                },
                right: DataPin {
                    pin_type: PinType::Pcm,
                    pin: Some("20".to_string()),
                    label: "GPIO 20 (PCM DIN)".to_string(),
                },
            },
            PinRow {
                left: DataPin {
                    pin_type: PinType::Gnd,
                    pin: None,
                    label: "Ground".to_string(),
                },
                right: DataPin {
                    pin_type: PinType::Pcm,
                    pin: Some("21".to_string()),
                    label: "GPIO 21 (PCM DOUT)".to_string(),
                },
            },
        ],
    }
}

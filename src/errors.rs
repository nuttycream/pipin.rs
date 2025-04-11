use std::{
    error::Error,
    fmt,
};

#[derive(Debug)]
pub enum GpioError {
    InvalidPin(i32),
    Setup,
    Direction(i32),
    Set(i32),
    Terminate,
    Device,
    NotInitialized,
    Clear(i32),
    PullDown(i32),
    PullUp(i32),
}

impl Error for GpioError {}

impl fmt::Display for GpioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpioError::Setup => write!(f, "Failed to Initialize"),
            GpioError::Direction(pin) => write!(f, "Failed to set direction {}", pin),
            GpioError::Set(pin) => write!(f, "Failed to set high - low {}", pin),
            GpioError::Terminate => write!(f, "Failed to terminate"),
            GpioError::Device => write!(f, "Failed to switch device"),
            GpioError::InvalidPin(pin) => write!(f, "Invalid gpio pin {}", pin),
            GpioError::NotInitialized => write!(f, "GPIO Not Initialized"),
            GpioError::Clear(pin) => write!(f, "Failed to clear GPIO {}", pin),
            GpioError::PullDown(pin) => write!(f, "Failed to set {} to pull down", pin),
            GpioError::PullUp(pin) => write!(f, "Failed to set {} to pull up", pin),
        }
    }
}

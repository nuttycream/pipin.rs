use std::{
    error::Error,
    fmt,
};

#[derive(Debug)]
pub enum GpioError {
    Init,
    Direction(i32),
    Set(i32),
    Terminate,
}

impl GpioError {}
impl Error for GpioError {}

impl fmt::Display for GpioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpioError::Init => write!(f, "Failed to Initialize"),
            GpioError::Direction(pin) => write!(f, "Failed to set direction {}", pin),
            GpioError::Set(pin) => write!(f, "Failed to set high - low {}", pin),
            GpioError::Terminate => write!(f, "Failed to terminate"),
        }
    }
}

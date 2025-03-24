use core::fmt;
use std::{error::Error, os::raw::c_int};

extern "C" {
    fn setup_io() -> c_int;
    fn terminate_io() -> c_int;

    fn set_gpio_out(gpio_pin: c_int) -> c_int;
    fn toggle_gpio(level: c_int, gpio_pin: c_int) -> c_int;
}

pub struct Gpio {
    pin: i32,
    initialized: bool,
}

#[derive(Debug)]
pub enum GpioError {
    Init,
    Direction(i32),
    Set(i32),
    Terminate,
}

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

impl Error for GpioError {}

impl Gpio {
    pub fn new(pin: i32) -> Result<Self, GpioError> {
        unsafe {
            if setup_io() != 0 {
                return Err(GpioError::Init);
            }
        }

        Ok(Gpio {
            pin,
            initialized: true,
        })
    }

    pub fn set_as_output(&self) -> Result<(), GpioError> {
        unsafe {
            if set_gpio_out(self.pin) == 0 {
                return Err(GpioError::Direction(self.pin));
            }
        }
        Ok(())
    }

    pub fn set_high(&self) -> Result<(), GpioError> {
        unsafe {
            if toggle_gpio(1, self.pin) != 0 {
                return Err(GpioError::Set(self.pin));
            }
        }
        Ok(())
    }

    pub fn set_low(&self) -> Result<(), GpioError> {
        unsafe {
            if toggle_gpio(0, self.pin) != 0 {
                return Err(GpioError::Set(self.pin));
            }
        }
        Ok(())
    }
}

impl Drop for Gpio {
    fn drop(&mut self) {
        if self.initialized {
            unsafe {
                terminate_io();
            }
            self.initialized = false;
        }
    }
}

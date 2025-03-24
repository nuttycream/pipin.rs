use crate::errors::GpioError;
use std::os::raw::c_int;

extern "C" {
    fn setup_io() -> c_int;
    fn terminate_io() -> c_int;

    fn set_gpio_out(gpio_pin: c_int) -> c_int;
    fn toggle_gpio(level: c_int, gpio_pin: c_int) -> c_int;
}

pub struct Gpio {
    initialized: bool,
}

trait GpioController: Sized {
    fn new() -> Result<Self, GpioError>;
    fn set_as_output(&self, pin: i32) -> Result<(), GpioError>;
    fn set_high(&self, pin: i32) -> Result<(), GpioError>;
    fn set_low(&self, pin: i32) -> Result<(), GpioError>;
}

impl GpioController for Gpio {
    fn new() -> Result<Self, GpioError> {
        unsafe {
            if setup_io() != 0 {
                return Err(GpioError::Init);
            }
        }

        Ok(Gpio { initialized: true })
    }

    fn set_as_output(&self, pin: i32) -> Result<(), GpioError> {
        unsafe {
            if set_gpio_out(pin) == 0 {
                return Err(GpioError::Direction(pin));
            }
        }
        Ok(())
    }

    fn set_high(&self, pin: i32) -> Result<(), GpioError> {
        unsafe {
            if toggle_gpio(1, pin) != 0 {
                return Err(GpioError::Set(pin));
            }
        }
        Ok(())
    }

    fn set_low(&self, pin: i32) -> Result<(), GpioError> {
        unsafe {
            if toggle_gpio(0, pin) != 0 {
                return Err(GpioError::Set(pin));
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

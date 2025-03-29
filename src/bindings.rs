use crate::errors::GpioError;
use std::os::raw::c_int;

unsafe extern "C" {
    fn setup_io() -> c_int;
    fn terminate_io() -> c_int;
    fn switch_hardware_address(option: c_int) -> c_int;

    fn set_gpio_out(gpio_pin: c_int) -> c_int;
    fn toggle_gpio(level: c_int, gpio_pin: c_int) -> c_int;
}

pub struct Gpio {
    initialized: bool,
    pin_status: [bool; 27],
}

pub trait GpioController: Sized {
    fn new() -> Self;
    fn setup(&mut self) -> Result<(), GpioError>;
    fn reset(&mut self) -> Result<(), GpioError>;
    fn switch_hardware(&self, option: i32) -> Result<(), GpioError>;
    fn set_as_output(&self, pin: i32) -> Result<(), GpioError>;
    fn set_high(&mut self, pin: i32) -> Result<(), GpioError>;
    fn set_low(&mut self, pin: i32) -> Result<(), GpioError>;
    fn toggle(&self, pin: i32) -> Result<bool, GpioError>;
    fn get_status(&self, pin: i32) -> bool;
}

impl GpioController for Gpio {
    fn new() -> Self {
        Gpio {
            initialized: false,
            pin_status: [false; 27],
        }
    }

    fn setup(&mut self) -> Result<(), GpioError> {
        unsafe {
            if setup_io() < 0 {
                return Err(GpioError::Setup);
            }

            self.initialized = true;
            // maybe its good to set all gpio pins to low here?
            self.reset()?;

            Ok(())
        }
    }

    fn reset(&mut self) -> Result<(), GpioError> {
        for pin in 0..27 {
            if self.pin_status[pin as usize] {
                self.set_low(pin)?;
            }
        }

        Ok(())
    }

    fn switch_hardware(&self, option: i32) -> Result<(), GpioError> {
        unsafe {
            if switch_hardware_address(option) < 0 {
                return Err(GpioError::Device);
            }
        }

        Ok(())
    }

    fn set_as_output(&self, pin: i32) -> Result<(), GpioError> {
        unsafe {
            if set_gpio_out(pin) < 0 {
                return Err(GpioError::Direction(pin));
            }
        }
        Ok(())
    }

    fn set_high(&mut self, pin: i32) -> Result<(), GpioError> {
        unsafe {
            if toggle_gpio(1, pin) < 0 {
                return Err(GpioError::Set(pin));
            }
        }
        Ok(())
    }

    fn set_low(&mut self, pin: i32) -> Result<(), GpioError> {
        unsafe {
            if toggle_gpio(0, pin) < 0 {
                return Err(GpioError::Set(pin));
            }
        }
        Ok(())
    }

    fn toggle(&self, pin: i32) -> Result<bool, GpioError> {
        Ok(true)
    }

    fn get_status(&self, pin: i32) -> bool {
        return false;
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

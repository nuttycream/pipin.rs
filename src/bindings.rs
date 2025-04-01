use crate::errors::GpioError;
use std::os::raw::c_int;

unsafe extern "C" {
    fn setup_io() -> c_int;
    fn terminate_io() -> c_int;
    fn switch_hardware_address(option: c_int) -> c_int;
    fn set_gpio_inp(gpio_pin: c_int) -> c_int;
    fn set_gpio_out(gpio_pin: c_int) -> c_int;
    fn get_gpio(gpio_pin: c_int) -> c_int;
    fn clear_gpio(gpio_pin: c_int) -> c_int;
    fn toggle_gpio(level: c_int, gpio_pin: c_int) -> c_int;
}

pub struct Gpio {
    initialized: bool,
    pin_status: [bool; 27],
}

pub trait GpioWrapper: Sized {
    fn new() -> Self;
    fn setup(&mut self) -> Result<(), GpioError>;
    fn reset(&mut self) -> Result<(), GpioError>;
    fn terminate(&mut self) -> Result<(), GpioError>;
    fn switch_hardware(&self, option: i32) -> Result<(), GpioError>;
    fn set_as_input(&self, pin: i32) -> Result<(), GpioError>;
    fn set_as_output(&self, pin: i32) -> Result<(), GpioError>;
    fn set_high(&mut self, pin: i32) -> Result<(), GpioError>;
    fn set_low(&mut self, pin: i32) -> Result<(), GpioError>;
    fn toggle(&mut self, pin: i32) -> Result<bool, GpioError>;
    fn get_gpio(&mut self, pin: i32) -> Result<bool, GpioError>;
    fn get_pin_status(&self, pin: i32) -> Result<bool, GpioError>;
    fn clear_gpio(&self, pin: i32) -> Result<(), GpioError>;
    fn is_initialized(&self) -> Result<bool, GpioError>;
}

impl GpioWrapper for Gpio {
    fn new() -> Self {
        Gpio {
            initialized: false,
            pin_status: [false; 27],
        }
    }

    fn setup(&mut self) -> Result<(), GpioError> {
        if self.initialized {
            return Ok(());
        }

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
        self.is_initialized()?;

        for pin in 0..27 {
            if self.pin_status[pin as usize] {
                self.clear_gpio(pin)?;
            }
        }

        Ok(())
    }

    fn terminate(&mut self) -> Result<(), GpioError> {
        self.is_initialized()?;

        unsafe {
            if terminate_io() < 0 {
                return Err(GpioError::Terminate);
            }
        }

        self.initialized = false;
        Ok(())
    }

    fn switch_hardware(&self, option: i32) -> Result<(), GpioError> {
        self.is_initialized()?;

        unsafe {
            if switch_hardware_address(option) < 0 {
                return Err(GpioError::Device);
            }
        }

        Ok(())
    }

    fn set_as_input(&self, pin: i32) -> Result<(), GpioError> {
        self.is_initialized()?;

        if pin < 0 || pin >= 27 {
            return Err(GpioError::InvalidPin(pin));
        }

        unsafe {
            if set_gpio_inp(pin) < 0 {
                return Err(GpioError::Direction(pin));
            }
        }
        Ok(())
    }

    fn set_as_output(&self, pin: i32) -> Result<(), GpioError> {
        self.is_initialized()?;

        if pin < 0 || pin >= 27 {
            return Err(GpioError::InvalidPin(pin));
        }

        unsafe {
            if set_gpio_out(pin) < 0 {
                return Err(GpioError::Direction(pin));
            }
        }
        Ok(())
    }

    fn set_high(&mut self, pin: i32) -> Result<(), GpioError> {
        self.is_initialized()?;

        if pin < 0 || pin >= 27 {
            return Err(GpioError::InvalidPin(pin));
        }
        unsafe {
            if toggle_gpio(1, pin) < 0 {
                return Err(GpioError::Set(pin));
            }
        }

        self.pin_status[pin as usize] = true;

        Ok(())
    }

    fn set_low(&mut self, pin: i32) -> Result<(), GpioError> {
        self.is_initialized()?;

        if pin < 0 || pin >= 27 {
            return Err(GpioError::InvalidPin(pin));
        }
        unsafe {
            if toggle_gpio(0, pin) < 0 {
                return Err(GpioError::Set(pin));
            }
        }

        self.pin_status[pin as usize] = false;

        Ok(())
    }

    fn toggle(&mut self, pin: i32) -> Result<bool, GpioError> {
        self.is_initialized()?;

        if pin < 0 || pin >= 27 {
            return Err(GpioError::Set(pin));
        }

        self.set_as_output(pin)?;

        let current_state = self.pin_status[pin as usize];
        let new_state = !current_state;

        if new_state {
            self.set_high(pin)?;
        } else {
            self.set_low(pin)?;
        }

        Ok(new_state)
    }

    fn get_gpio(&mut self, pin: i32) -> Result<bool, GpioError> {
        self.is_initialized()?;

        if pin >= 0 && pin < 27 {
            self.set_as_input(pin)?;
            unsafe {
                if get_gpio(pin) == 0 {
                    return Ok(true);
                } else {
                    return Ok(false);
                }
            }
        } else {
            return Err(GpioError::InvalidPin(pin));
        }
    }

    fn get_pin_status(&self, pin: i32) -> Result<bool, GpioError> {
        self.is_initialized()?;

        if pin >= 0 && pin < 27 {
            Ok(self.pin_status[pin as usize])
        } else {
            return Err(GpioError::InvalidPin(pin));
        }
    }

    fn clear_gpio(&self, pin: i32) -> Result<(), GpioError> {
        self.is_initialized()?;

        if pin >= 0 && pin < 27 {
            unsafe {
                if clear_gpio(pin) < 0 {
                    return Err(GpioError::Clear(pin));
                }
            }
        } else {
            return Err(GpioError::InvalidPin(pin));
        }

        Ok(())
    }

    fn is_initialized(&self) -> Result<bool, GpioError> {
        if !self.initialized {
            return Err(GpioError::NotInitialized);
        }
        Ok(true)
    }
}

impl Drop for Gpio {
    fn drop(&mut self) {
        let _ = self.reset();

        // we dont need to handle the error
        // here since failing at munmap()
        // means mmap() wasn't executed
        // so do nothing... and drop still
        unsafe {
            terminate_io();
        }
    }
}

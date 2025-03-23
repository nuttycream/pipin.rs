use std::os::raw::c_int;

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

impl Gpio {
    pub fn new(pin: i32) -> Result<Self, &'static str> {
        unsafe {
            if setup_io() != 0 {
                return Err("Failed to initialize GPIO");
            }
        }

        Ok(Gpio {
            pin,
            initialized: true,
        })
    }

    pub fn set_as_output(&self) -> Result<(), &'static str> {
        unsafe {
            if set_gpio_out(self.pin) == 0 {
                return Err("Failed to set GPIO as output");
            }
        }
        Ok(())
    }

    pub fn set_high(&self) -> Result<(), &'static str> {
        unsafe {
            if toggle_gpio(1, self.pin) != 0 {
                return Err("Failed to set GPIO high");
            }
        }
        Ok(())
    }

    pub fn set_low(&self) -> Result<(), &'static str> {
        unsafe {
            if toggle_gpio(0, self.pin) != 0 {
                return Err("Failed to set GPIO low");
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

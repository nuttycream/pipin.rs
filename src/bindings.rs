use crate::errors::GpioError;
use std::os::raw::c_int;

unsafe extern "C" {
    fn setup_gpio() -> c_int;
    fn terminate_gpio() -> c_int;
    fn switch_hardware_address(option: c_int) -> c_int;
    fn detect_peripheral_base() -> c_int;
    fn set_gpio_direction(
        direction: c_int,
        gpio_pin: c_int,
    ) -> c_int;
    fn get_gpio(gpio_pin: c_int) -> c_int;
    fn write_gpio(level: c_int, gpio_pin: c_int) -> c_int;
    fn set_gpio_pull(
        direction: c_int,
        gpio_pin: c_int,
        wait_time: c_int,
    ) -> c_int;
}

pub struct Gpio {
    device: i32,
    initialized: bool,
    pin_status: [bool; 28],
}

pub trait GpioWrapper: Sized {
    fn new() -> Self;
    fn setup(&mut self) -> Result<(), GpioError>;
    fn reset(&mut self) -> Result<(), GpioError>;
    fn terminate(&mut self) -> Result<(), GpioError>;
    fn switch_device(
        &mut self,
        device: i32,
    ) -> Result<(), GpioError>;
    fn set_as_input(
        &self,
        pin: i32,
    ) -> Result<(), GpioError>;
    fn set_as_output(
        &self,
        pin: i32,
    ) -> Result<(), GpioError>;
    fn set_high(
        &mut self,
        pin: i32,
    ) -> Result<(), GpioError>;
    fn set_low(
        &mut self,
        pin: i32,
    ) -> Result<(), GpioError>;
    fn toggle(
        &mut self,
        pin: i32,
    ) -> Result<bool, GpioError>;
    fn get_gpio(
        &mut self,
        pin: i32,
    ) -> Result<bool, GpioError>;
    fn clear_gpio(&self, pin: i32)
        -> Result<(), GpioError>;
    fn set_pulldown(
        &self,
        pin: i32,
        wait_time: i32,
    ) -> Result<(), GpioError>;
    fn set_pullup(
        &self,
        pin: i32,
        wait_time: i32,
    ) -> Result<(), GpioError>;
    fn validate_inp(
        &self,
        pin: i32,
    ) -> Result<i32, GpioError>;
}

impl GpioWrapper for Gpio {
    fn new() -> Self {
        Gpio {
            device: 0,
            initialized: false,
            pin_status: [false; 28],
        }
    }

    fn validate_inp(
        &self,
        pin: i32,
    ) -> Result<i32, GpioError> {
        if !self.initialized {
            return Err(GpioError::NotInitialized);
        }

        if !(0..=27).contains(&pin) {
            return Err(GpioError::InvalidPin(pin));
        }

        Ok(pin)
    }

    fn setup(&mut self) -> Result<(), GpioError> {
        if self.initialized {
            println!("already initialized");
            return Ok(());
        }

        unsafe {
            println!("attempting to detect peripheral");
            if detect_peripheral_base() < 0 {
                return Err(GpioError::HardwareDetection);
            }
        }

        unsafe {
            if setup_gpio() < 0 {
                return Err(GpioError::Setup);
            }

            self.initialized = true;
            // maybe its good to set all gpio pins to low here?
            self.reset()?;

            Ok(())
        }
    }

    fn reset(&mut self) -> Result<(), GpioError> {
        if !self.initialized {
            return Err(GpioError::NotInitialized);
        }

        for pin in 0..27 {
            if self.pin_status[pin as usize] {
                self.clear_gpio(pin)?;
            }
        }

        Ok(())
    }

    fn terminate(&mut self) -> Result<(), GpioError> {
        if !self.initialized {
            return Err(GpioError::NotInitialized);
        }

        unsafe {
            if terminate_gpio() < 0 {
                return Err(GpioError::Terminate);
            }
        }

        self.initialized = false;
        Ok(())
    }

    fn switch_device(
        &mut self,
        device: i32,
    ) -> Result<(), GpioError> {
        if !(0..=3).contains(&device) {
            return Err(GpioError::InvalidDevice(device));
        }

        unsafe {
            if switch_hardware_address(device) < 0 {
                return Err(GpioError::SwitchDevice(
                    device,
                ));
            }
        }

        self.device = device;

        // gotta re - setup
        self.initialized = false;
        self.setup()?;

        Ok(())
    }

    fn set_as_input(
        &self,
        pin: i32,
    ) -> Result<(), GpioError> {
        self.validate_inp(pin)?;

        unsafe {
            if set_gpio_direction(0, pin) < 0 {
                return Err(GpioError::Direction(pin));
            }
        }
        Ok(())
    }

    fn set_as_output(
        &self,
        pin: i32,
    ) -> Result<(), GpioError> {
        self.validate_inp(pin)?;

        unsafe {
            if set_gpio_direction(1, pin) < 0 {
                return Err(GpioError::Direction(pin));
            }
        }
        Ok(())
    }

    fn set_high(
        &mut self,
        pin: i32,
    ) -> Result<(), GpioError> {
        self.validate_inp(pin)?;

        unsafe {
            if write_gpio(1, pin) < 0 {
                return Err(GpioError::Set(pin));
            }
        }

        self.pin_status[pin as usize] = true;

        Ok(())
    }

    fn set_low(
        &mut self,
        pin: i32,
    ) -> Result<(), GpioError> {
        self.validate_inp(pin)?;

        unsafe {
            if write_gpio(0, pin) < 0 {
                return Err(GpioError::Set(pin));
            }
        }

        self.pin_status[pin as usize] = false;

        Ok(())
    }

    fn toggle(
        &mut self,
        pin: i32,
    ) -> Result<bool, GpioError> {
        self.validate_inp(pin)?;

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

    fn get_gpio(
        &mut self,
        pin: i32,
    ) -> Result<bool, GpioError> {
        self.validate_inp(pin)?;

        self.set_as_input(pin)?;
        unsafe {
            if get_gpio(pin) == 1 {
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }

    fn clear_gpio(
        &self,
        pin: i32,
    ) -> Result<(), GpioError> {
        self.validate_inp(pin)?;

        unsafe {
            if write_gpio(0, pin) < 0 {
                return Err(GpioError::Clear(pin));
            }
        }

        Ok(())
    }

    fn set_pulldown(
        &self,
        pin: i32,
        wait_time: i32,
    ) -> Result<(), GpioError> {
        self.validate_inp(pin)?;

        unsafe {
            if set_gpio_pull(1, pin, wait_time) < 0 {
                return Err(GpioError::PullDown(pin));
            }
        }

        Ok(())
    }

    fn set_pullup(
        &self,
        pin: i32,
        wait_time: i32,
    ) -> Result<(), GpioError> {
        self.validate_inp(pin)?;

        unsafe {
            if set_gpio_pull(0, pin, wait_time) < 0 {
                return Err(GpioError::PullUp(pin));
            }
        }

        Ok(())
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
            terminate_gpio();
        }
    }
}

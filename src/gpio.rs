use std::{
    fs::OpenOptions,
    num::NonZero,
    os::unix::fs::OpenOptionsExt,
    ptr::NonNull,
    sync::atomic::{AtomicPtr, Ordering},
};

use crate::errors::GpioError;
use nix::{
    libc,
    sys::mman::{mmap, munmap, MapFlags, ProtFlags},
};

// https://pinout.xyz/
#[derive(Copy, Clone, Debug)]
pub enum PinType {
    Power5v,  // red
    Power3v3, // orange
    Gnd,      // black

    Gpio, // green
    I2c,  // blue
    Spi,  // pink
    Uart, // purple
    Pcm,  // yellow
}

#[derive(Copy, Clone, Debug)]
pub enum PullType {
    None,
    Up,
    Down,
}

#[derive(Copy, Clone, Debug)]
pub enum PinDirection {
    Input,
    Output,
}

#[derive(Copy, Clone, Debug)]
pub enum PinLevel {
    High,
    Low,
}

#[derive(Copy, Clone, Debug)]
pub struct Pin {
    pin_type: PinType,
    pull: PullType,
    level: PinLevel,
    direction: PinDirection,
}

pub struct Gpio {
    gpio_map: Option<AtomicPtr<u32>>,
    device: i64,
    initialized: bool,
    pins: Vec<Pin>,
}

impl Gpio {
    pub fn new() -> Self {
        Gpio {
            gpio_map: None,
            device: 0,
            initialized: false,
            pins: Vec::new(),
        }
    }

    fn validate_input(&self, pin: i32) -> Result<i32, GpioError> {
        if !self.initialized {
            return Err(GpioError::NotInitialized);
        }

        if !(0..=27).contains(&pin) {
            return Err(GpioError::InvalidPin(pin));
        }

        Ok(pin)
    }

    // https://stackoverflow.com/a/44510388/17123405

    pub fn setup(&mut self) -> Result<(), GpioError> {
        if self.initialized {
            println!("GPIO device already initialized.");
            return Ok(());
        }

        unsafe {
            let block_size = match NonZero::new(4096) {
                Some(val) => val,
                None => {
                    println!("Somehow failed to create NonZero BLOCKSIZE");
                    return Err(GpioError::Setup);
                }
            };

            // 0 since we're using
            // /dev/gpiomem
            // if we want manual control,
            // we'll have to
            // use /dev/mem and
            // detect_peripheral_address()
            let gpio_address = 0;

            let dev_mem = match OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(libc::O_SYNC)
                .open("/dev/gpiomem")
            {
                Ok(dev_mem) => {
                    println!("Opened /dev/gpiomem");
                    dev_mem
                }
                Err(e) => {
                    println!("Failed to open /dev/gpiomem: {e}");
                    return Err(GpioError::Setup);
                }
            };

            match mmap(
                None,
                block_size,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                MapFlags::MAP_SHARED,
                dev_mem,
                gpio_address,
            ) {
                Ok(map) => {
                    println!(
                        "Memory mapped successfully -> casting to gpio_map"
                    );

                    // is this safe?
                    let ptr = map.cast::<u32>().as_ptr();
                    self.gpio_map = Some(AtomicPtr::new(ptr));
                    println!("Casted successfully");
                }
                Err(e) => {
                    println!("Failed to mmap: {e}");
                    return Err(GpioError::Setup);
                }
            };
        }

        self.initialized = true;
        Ok(())
    }

    pub fn detect_hardware_address(&self) -> Result<u64, GpioError> {
        // fallback for pi zero 2w
        // 0x3f20000
        // fallback for pi 4
        // 0xfe20000
        Ok(0x3f20000)
    }

    pub fn reset(&mut self) -> Result<(), GpioError> {
        if !self.initialized {
            return Err(GpioError::NotInitialized);
        }

        Ok(())
    }

    pub fn terminate(&mut self) -> Result<(), GpioError> {
        if !self.initialized {
            return Err(GpioError::NotInitialized);
        }

        Ok(())
    }

    pub fn set_direction(
        &mut self,
        pin: i32,
        direction: PinDirection,
    ) -> Result<(), GpioError> {
        self.validate_input(pin)?;
        Ok(())
    }

    pub fn set_level(
        &mut self,
        pin: i32,
        level: PinLevel,
    ) -> Result<(), GpioError> {
        self.validate_input(pin)?;
        Ok(())
    }

    pub fn set_pull_type(
        &mut self,
        pin: i32,
        pull_type: PullType,
    ) -> Result<(), GpioError> {
        self.validate_input(pin)?;
        Ok(())
    }

    pub fn get_level(&self, pin: i32) -> Result<PinLevel, GpioError> {
        self.validate_input(pin)?;
        match self.pins[pin as usize].level {
            PinLevel::High => Ok(PinLevel::High),
            PinLevel::Low => Ok(PinLevel::Low),
        }
    }

    pub fn get_direction(&self, pin: i32) -> Result<PinDirection, GpioError> {
        self.validate_input(pin)?;
        match self.pins[pin as usize].direction {
            PinDirection::Input => Ok(PinDirection::Input),
            PinDirection::Output => Ok(PinDirection::Output),
        }
    }
}

impl Drop for Gpio {
    fn drop(&mut self) {
        if self.initialized {
            if let Some(atomic_ptr) = &self.gpio_map {
                unsafe {
                    let ptr = atomic_ptr.load(Ordering::SeqCst);
                    if !ptr.is_null() {
                        if let Some(non_null) = NonNull::new(ptr as *mut _) {
                            munmap(non_null.cast(), 4096).ok();
                            println!("Unmapping memory")
                        }
                    }
                }
            }
        }
    }
}

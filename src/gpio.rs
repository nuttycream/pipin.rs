use crate::errors::GpioError;
use nix::{
    libc::O_SYNC,
    sys::mman::{mmap, munmap, MapFlags, ProtFlags},
};
use std::{
    fs::OpenOptions,
    num::NonZero,
    os::unix::fs::OpenOptionsExt,
    ptr::{read_volatile, write_volatile, NonNull},
    sync::atomic::{AtomicPtr, Ordering},
    thread::sleep,
    time::Duration,
};

const BLOCK_SIZE: usize = 4096;
const GPIO_SET_OFFSET: usize = 7;
const GPIO_CLR_OFFSET: usize = 10;
const GPIO_LEV_OFFSET: usize = 13;
const GPIO_PULL_OFFSET: usize = 37;
const GPIO_PULLCLK0_OFFSET: usize = 38;

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
    None = 0,
    Down = 1,
    Up = 2,
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
    initialized: bool,
    pins: [Pin; 28],
}

impl Gpio {
    pub fn new() -> Self {
        Gpio {
            gpio_map: None,
            initialized: false,
            pins: [Pin {
                pin_type: PinType::Gpio,
                pull: PullType::None,
                level: PinLevel::Low,
                direction: PinDirection::Input,
            }; 28],
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
    // helper func to read a volatile register
    unsafe fn read_register(&self, offset: usize) -> Result<u32, GpioError> {
        if !self.initialized || self.gpio_map.is_none() {
            return Err(GpioError::NotInitialized);
        }

        // get from the base pointer and
        // add the offset
        if let Some(atomic_ptr) = &self.gpio_map {
            let base = atomic_ptr.load(Ordering::SeqCst);
            let reg = base.add(offset);

            // then read
            Ok(read_volatile(reg))
        } else {
            Err(GpioError::NotInitialized)
        }
    }

    // helper func to write a volatile register
    unsafe fn write_register(
        &self,
        offset: usize,
        value: u32,
    ) -> Result<(), GpioError> {
        if !self.initialized || self.gpio_map.is_none() {
            return Err(GpioError::NotInitialized);
        }

        // get from the base ptr and
        // add the offset
        if let Some(atomic_ptr) = &self.gpio_map {
            let base = atomic_ptr.load(Ordering::SeqCst);
            let reg = base.add(offset);

            // write
            write_volatile(reg, value);

            Ok(())
        } else {
            Err(GpioError::NotInitialized)
        }
    }

    pub fn setup(&mut self) -> Result<(), GpioError> {
        if self.initialized {
            println!("GPIO device already initialized.");
            return Ok(());
        }

        unsafe {
            let block_size = match NonZero::new(BLOCK_SIZE) {
                Some(val) => val,
                None => {
                    println!("Somehow failed to create NonZero BLOCK_SIZE");
                    return Err(GpioError::Setup);
                }
            };

            // 0 since we're using
            // /dev/gpiomem
            // if we want manual control,
            // we'll have to
            // use /dev/mem and
            // detect_peripheral_address()
            // but requires sudo
            // rather than gpio group
            // `groups`
            // `sudo usermod -a -G gpio <user>`
            let gpio_address = 0;

            let dev_mem = match OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(O_SYNC)
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

                    // AtomicPtr since NonNull cant be
                    // shared across
                    // tokio threads
                    let ptr = map.cast::<u32>().as_ptr();
                    self.gpio_map = Some(AtomicPtr::new(ptr));

                    println!("Finished init gpio_map");
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

    /* to be deprecated
    pub fn detect_hardware_address(&self) -> Result<u64, GpioError> {
        // cat /proc/iomem | grep gpio
        // otherwise
        // fallback for pi zero 2w
        // 0x3f20000
        // fallback for pi 4
        // 0xfe20000
        Ok(0x3f20000)
    }
    */

    pub fn reset(&mut self) -> Result<(), GpioError> {
        if !self.initialized {
            return Err(GpioError::NotInitialized);
        }

        // reseting all pins to input
        for pin in 0..28 {
            self.set_direction(pin, PinDirection::Input)?;
            self.set_level(pin, PinLevel::Low)?;
            self.set_pull_type(pin, PullType::None)?;
        }

        Ok(())
    }

    pub fn terminate(&mut self) -> Result<(), GpioError> {
        if !self.initialized {
            return Err(GpioError::NotInitialized);
        }

        // should we reset all pins before terminating?
        // self.reset()?;

        if let Some(atomic_ptr) = &self.gpio_map {
            unsafe {
                let ptr = atomic_ptr.load(Ordering::SeqCst);
                if !ptr.is_null() {
                    if let Some(non_null) = NonNull::new(ptr as *mut _) {
                        munmap(non_null.cast(), BLOCK_SIZE).ok();
                        println!("Unmapping memory on terminate");
                    }
                }
            }
        }

        self.gpio_map = None;
        self.initialized = false;

        Ok(())
    }

    // wrapper for set_level() for toggling
    pub fn toggle(&mut self, pin: i32) -> Result<PinLevel, GpioError> {
        self.validate_input(pin)?;

        let current_level = self.get_level(pin)?;

        match current_level {
            PinLevel::High => {
                self.set_level(pin, PinLevel::Low)?;
                Ok(PinLevel::Low)
            }
            PinLevel::Low => {
                self.set_level(pin, PinLevel::High)?;
                Ok(PinLevel::High)
            }
        }
    }

    pub fn set_direction(
        &mut self,
        pin: i32,
        direction: PinDirection,
    ) -> Result<(), GpioError> {
        self.validate_input(pin)?;

        unsafe {
            let reg = (pin / 10) as usize;
            let bit = ((pin % 10) * 3) as usize;

            // read
            let mut reg_value = self.read_register(reg)?;

            // clear
            reg_value &= !(7 << bit);

            // then set bits based on dir
            match direction {
                PinDirection::Output => {
                    reg_value |= 1 << bit;
                }
                PinDirection::Input => {
                    // should do nothing here
                    // since clearing it
                }
            }

            // wriet it back
            self.write_register(reg, reg_value)?;
        }

        self.pins[pin as usize].direction = direction;
        Ok(())
    }

    pub fn set_level(
        &mut self,
        pin: i32,
        level: PinLevel,
    ) -> Result<(), GpioError> {
        self.validate_input(pin)?;

        // set to output first
        // i assume this will only be triggered
        // when we want to output a signal no?
        self.set_direction(pin, PinDirection::Output)?;

        unsafe {
            match level {
                PinLevel::High => {
                    println!("Setting Pin Level to High");
                    self.write_register(GPIO_SET_OFFSET, 1 << pin)?;
                }
                PinLevel::Low => {
                    println!("Setting Pin Level to Low");
                    self.write_register(GPIO_CLR_OFFSET, 1 << pin)?;
                }
            }
        }

        self.pins[pin as usize].level = level;
        Ok(())
    }

    pub fn set_pull_type(
        &mut self,
        pin: i32,
        pull_type: PullType,
    ) -> Result<(), GpioError> {
        self.validate_input(pin)?;
        // todo: let's make this an optional param
        let wait_time = 100;

        unsafe {
            // clear
            self.write_register(GPIO_PULL_OFFSET, 0)?;

            // use std sleep
            // should be good enough hopefully
            sleep(Duration::from_micros(wait_time));

            // now pull
            self.write_register(GPIO_PULL_OFFSET, pull_type as u32)?;
            sleep(Duration::from_micros(wait_time));

            // clock it if not none
            match pull_type {
                PullType::None => println!("Not going to clock for NONE"),
                PullType::Down | PullType::Up => {
                    self.write_register(GPIO_PULLCLK0_OFFSET, 1 << pin)?;
                    sleep(Duration::from_micros(wait_time));
                }
            }

            // then clear again
            self.write_register(GPIO_PULL_OFFSET, 0)?;
            self.write_register(GPIO_PULLCLK0_OFFSET, 0)?;
        }

        self.pins[pin as usize].pull = pull_type;
        Ok(())
    }

    pub fn get_level(&self, pin: i32) -> Result<PinLevel, GpioError> {
        self.validate_input(pin)?;
        /*
        * should only used to get current frontend state imo
        * should i separate this into its own?
        match self.pins[pin as usize].level {
            PinLevel::High => return Ok(PinLevel::High),
            PinLevel::Low => return Ok(PinLevel::Low),
        };
        */

        unsafe {
            let level = self.read_register(GPIO_LEV_OFFSET)?;
            if (level & (1 << pin)) != 0 {
                Ok(PinLevel::High)
            } else {
                Ok(PinLevel::Low)
            }
        }
    }

    // get whether pin is set to output or input
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
        if !self.initialized {
            return;
        }

        let atomic_ptr = match &self.gpio_map {
            Some(ptr) => ptr,
            None => return,
        };

        let ptr = atomic_ptr.load(Ordering::SeqCst);
        if ptr.is_null() {
            return;
        }

        unsafe {
            if let Some(non_null) = NonNull::new(ptr as *mut _) {
                munmap(non_null.cast(), BLOCK_SIZE).ok();
                println!("Unmapping memory")
            }
        }
    }
}

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

#[derive(Clone, Debug)]
pub enum PinColumn {
    Left,
    Right,
}

#[derive(Clone, Debug)]
pub struct Pin {
    // ui stuff
    pub number: Option<i32>,
    pub pin_type: PinType,
    pub label: String,
    pub column: PinColumn,

    //physical state
    pull: PullType,
    level: PinLevel,
    direction: PinDirection,
}

pub struct Gpio {
    pub initialized: bool,
    pub pins: Vec<Pin>,

    gpio_map: Option<AtomicPtr<u32>>,
}

impl Gpio {
    pub fn new() -> Self {
        Gpio {
            gpio_map: None,
            initialized: false,
            pins: default_pins(),
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

        /*
        for pin_data in default_pins() {
            self.pins.insert(pin_data.number, pin_data);
        }
        */

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

        for p in &mut self.pins {
            if let Some(num) = p.number {
                if num == pin {
                    p.direction = direction;
                    break;
                }
            }
        }

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

        // pins aren't mapped 1 to 1 on
        // physical pins to the vector index
        // so using something like pins[pin]
        // fails and sets the wrong
        // pin idx
        // todo: better way to handle this
        for p in &mut self.pins {
            if let Some(num) = p.number {
                if num == pin {
                    p.level = level;
                    break;
                }
            }
        }

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

        for p in &mut self.pins {
            if let Some(num) = p.number {
                if num == pin {
                    p.pull = pull_type;
                    break;
                }
            }
        }

        Ok(())
    }

    pub fn get_level(&self, pin: i32) -> Result<PinLevel, GpioError> {
        self.validate_input(pin)?;

        unsafe {
            let level = self.read_register(GPIO_LEV_OFFSET)?;
            if (level & (1 << pin)) != 0 {
                Ok(PinLevel::High)
            } else {
                Ok(PinLevel::Low)
            }
        }
    }

    // do we realistically need this?
    // assuming in the future we need to determine direction
    // at a quick glance for the frontend
    pub fn _get_direction(&self, pin: i32) -> Result<PinDirection, GpioError> {
        self.validate_input(pin)?;
        match self.pins[pin as usize].direction {
            PinDirection::Input => Ok(PinDirection::Input),
            PinDirection::Output => Ok(PinDirection::Output),
        }
    }

    pub fn get_html_pins(&self) -> Result<String, GpioError> {
        let left_pins: Vec<&Pin> = self
            .pins
            .iter()
            .filter(|pin| matches!(pin.column, PinColumn::Left))
            .collect();

        let right_pins: Vec<&Pin> = self
            .pins
            .iter()
            .filter(|pin| matches!(pin.column, PinColumn::Right))
            .collect();

        let mut html = String::from("<div class=\"gpio-layout\">");

        for row in 0..20 {
            html.push_str("<div class=\"gpio-row\">");

            //lefty
            html.push_str(&self.render_pin(left_pins[row])?);

            //righty
            html.push_str(&self.render_pin(right_pins[row])?);

            html.push_str("</div>");
        }

        html.push_str("</div>");
        Ok(html)
    }

    fn render_pin(&self, pin: &Pin) -> Result<String, GpioError> {
        let level = match pin.level {
            PinLevel::High => "high",
            PinLevel::Low => "low",
        };

        let checked = match pin.level {
            PinLevel::High => "checked",
            PinLevel::Low => "",
        };

        let powered = match pin.pin_type {
            PinType::Power5v | PinType::Power3v3 => "power",
            PinType::Gnd => "ground",
            _ => "gpio",
        };

        if powered == "gpio" {
            let pin_num = pin.number.unwrap();
            let unique_id = format!("gpio-pin-{}", pin_num);

            let ret = format!(
                r#"
            <div id="{0}" class="pin-wrapper">
            <label class="toggle-switch">
            <input type="checkbox" {5} id="checkbox-{1}" 
                class="pin-checkbox" ws-send
                hx-trigger="change" hx-vals='{{"pin": "{1}"}}'>
            <span class="pin {2} {3}">{4}</span>
            </label>
            </div>
         "#,
                unique_id, pin_num, powered, level, pin.label, checked
            );
            Ok(ret)
        } else {
            let ret = format!(
                r#"<div class="pin-wrapper">
                    <span class="pin {0}" disabled>{1}</span>
                    </div>"#,
                powered, pin.label
            );
            Ok(ret)
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

// ideally this would never change
// since we're only supporting pi's
// so idk why i had it as part of config
// i guess if a user wants to move it around
// but idk if that's a good feature
pub fn default_pins() -> Vec<Pin> {
    let mut pins = Vec::new();

    let pin = |number: Option<i32>,
               column: PinColumn,
               pin_type: PinType,
               label: &str| Pin {
        number,
        column,
        pin_type,
        label: label.to_string(),
        level: PinLevel::Low,
        direction: PinDirection::Input,
        pull: PullType::None,
    };

    pins.push(pin(None, PinColumn::Left, PinType::Power3v3, "3v3 Power"));
    pins.push(pin(
        Some(2),
        PinColumn::Left,
        PinType::I2c,
        "GPIO 2 (I2C1 SDA)",
    ));
    pins.push(pin(
        Some(3),
        PinColumn::Left,
        PinType::I2c,
        "GPIO 3 (I2C1 SCL)",
    ));
    pins.push(pin(
        Some(4),
        PinColumn::Left,
        PinType::Gpio,
        "GPIO 4 (GPCLK0)",
    ));
    pins.push(pin(None, PinColumn::Left, PinType::Gnd, "Ground"));
    pins.push(pin(Some(17), PinColumn::Left, PinType::Gpio, "GPIO 17"));
    pins.push(pin(Some(27), PinColumn::Left, PinType::Gpio, "GPIO 27"));
    pins.push(pin(Some(22), PinColumn::Left, PinType::Gpio, "GPIO 22"));
    pins.push(pin(None, PinColumn::Left, PinType::Power3v3, "3v3 Power"));
    pins.push(pin(
        Some(10),
        PinColumn::Left,
        PinType::Spi,
        "GPIO 10 (SPI0 MOSI)",
    ));
    pins.push(pin(
        Some(9),
        PinColumn::Left,
        PinType::Spi,
        "GPIO 9 (SPI0 MISO)",
    ));
    pins.push(pin(
        Some(11),
        PinColumn::Left,
        PinType::Spi,
        "GPIO 11 (SPI0 SCLK)",
    ));
    pins.push(pin(None, PinColumn::Left, PinType::Gnd, "Ground"));
    pins.push(pin(
        Some(0),
        PinColumn::Left,
        PinType::I2c,
        "GPIO 0 (EEPROM SDA)",
    ));
    pins.push(pin(Some(5), PinColumn::Left, PinType::Gpio, "GPIO 5"));
    pins.push(pin(Some(6), PinColumn::Left, PinType::Gpio, "GPIO 6"));
    pins.push(pin(
        Some(13),
        PinColumn::Left,
        PinType::Gpio,
        "GPIO 13 (PWM1)",
    ));
    pins.push(pin(
        Some(19),
        PinColumn::Left,
        PinType::Pcm,
        "GPIO 19 (PCM FS)",
    ));
    pins.push(pin(Some(26), PinColumn::Left, PinType::Gpio, "GPIO 26"));
    pins.push(pin(None, PinColumn::Left, PinType::Gnd, "Ground"));

    pins.push(pin(None, PinColumn::Right, PinType::Power5v, "5v Power"));
    pins.push(pin(None, PinColumn::Right, PinType::Power5v, "5v Power"));
    pins.push(pin(None, PinColumn::Right, PinType::Gnd, "Ground"));
    pins.push(pin(
        Some(14),
        PinColumn::Right,
        PinType::Uart,
        "GPIO 14 (UART TX)",
    ));
    pins.push(pin(
        Some(15),
        PinColumn::Right,
        PinType::Uart,
        "GPIO 15 (UART RX)",
    ));
    pins.push(pin(
        Some(18),
        PinColumn::Right,
        PinType::Pcm,
        "GPIO 18 (PCM CLK)",
    ));
    pins.push(pin(None, PinColumn::Right, PinType::Gnd, "Ground"));
    pins.push(pin(Some(23), PinColumn::Right, PinType::Gpio, "GPIO 23"));
    pins.push(pin(Some(24), PinColumn::Right, PinType::Gpio, "GPIO 24"));
    pins.push(pin(None, PinColumn::Right, PinType::Gnd, "Ground"));
    pins.push(pin(Some(25), PinColumn::Right, PinType::Gpio, "GPIO 25"));
    pins.push(pin(
        Some(8),
        PinColumn::Right,
        PinType::Spi,
        "GPIO 8 (SPI0 CE0)",
    ));
    pins.push(pin(
        Some(7),
        PinColumn::Right,
        PinType::Spi,
        "GPIO 7 (SPI0 CE1)",
    ));
    pins.push(pin(
        Some(1),
        PinColumn::Right,
        PinType::I2c,
        "GPIO 1 (EEPROM SCL)",
    ));
    pins.push(pin(None, PinColumn::Right, PinType::Gnd, "Ground"));
    pins.push(pin(
        Some(12),
        PinColumn::Right,
        PinType::Gpio,
        "GPIO 12 (PWM0)",
    ));
    pins.push(pin(None, PinColumn::Right, PinType::Gnd, "Ground"));
    pins.push(pin(Some(16), PinColumn::Right, PinType::Gpio, "GPIO 16"));
    pins.push(pin(
        Some(20),
        PinColumn::Right,
        PinType::Pcm,
        "GPIO 20 (PCM DIN)",
    ));
    pins.push(pin(
        Some(21),
        PinColumn::Right,
        PinType::Pcm,
        "GPIO 21 (PCM DOUT)",
    ));

    pins
}

//use axum::{routing::get, Router};
mod bindings;
use bindings::Gpio;

use std::{thread, time::Duration};

fn main() -> Result<(), &'static str> {
    println!("starting");

    let gpio = Gpio::new(17)?;

    gpio.set_as_output()?;

    for _i in 0..5 {
        println!("on");
        gpio.set_high()?;
        thread::sleep(Duration::from_millis(500));

        println!("off");
        gpio.set_low()?;
        thread::sleep(Duration::from_millis(500));
    }

    println!("done");
    Ok(())
}

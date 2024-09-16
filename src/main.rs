//! this example test the RP Pico W on board LED.
//!
//! It does not work with the RP Pico board.

#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

mod cyw43_driver;

const CYCLE: u64 = 833;

async fn send_command<'a>(pin: &mut Output<'a>, command: u8) {
    // Pin is set to high and when low for 8 cycles it signifies a start of a command
    pin.set_low();
    Timer::after(Duration::from_micros(CYCLE * 8)).await;

    //Convert the command to the 8 bit binary representation
    for i in (0..8).rev() {
        let bit = (command >> i) & 1;
        println!("bit: {}", bit);
        // Send the high-low sequence based on the bit value
        match bit {
            1 => {
                pin.set_high();
                Timer::after(Duration::from_micros(CYCLE * 4)).await;
                pin.set_low();
                Timer::after(Duration::from_micros(CYCLE * 1)).await;
            }
            0 => {
                pin.set_high();
                Timer::after(Duration::from_micros(CYCLE * 1)).await;
                pin.set_low();
                Timer::after(Duration::from_micros(CYCLE * 1)).await;
            }
            2_u8..=u8::MAX => error!("Invalid bit value"),
        }
        //TODO test if this is needed
        pin.set_low();
        Timer::after(Duration::from_micros(CYCLE * 1)).await;
    }

    // Set back to high to end the transmission
    pin.set_high();
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let delay = Duration::from_secs(5);
    let mut pin = Output::new(p.PIN_16, Level::High);

    pin.set_high();
    //wake up
    // send_command(&mut pin, 0xB1).await;

    loop {
        info!("Tilt Left!");
        send_command(&mut pin, 0x8B).await;
        Timer::after(Duration::from_secs(3)).await;
        info!("Tilt Right!");
        send_command(&mut pin, 0x83).await;
        Timer::after(delay).await;
    }
}

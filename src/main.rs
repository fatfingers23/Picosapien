//! this example test the RP Pico W on board LED.
//!
//! It does not work with the RP Pico board.

#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::clocks;
use embassy_rp::gpio::{AnyPin, Level, Output, Pin};
use embassy_rp::pwm;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

mod cyw43_driver;

const CYCLE: u64 = 833;
const PERIOD: Duration = Duration::from_micros(CYCLE);

const CODE_RSTurnRight: u8 = 0x80;
const CODE_RSRightArmUp: u8 = 0x81;

async fn send_command<'a>(pin: &mut Output<'a>, command: u8) {
    // Send start bit (wf_head)
    pin.set_low();
    Timer::after(Duration::from_micros(CYCLE * 8)).await;

    // Send 8 data bits, most significant bit (MSB) first
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
        pin.set_low();
        Timer::after(Duration::from_micros(CYCLE * 1)).await;
    }

    // Send end bit (wf_tail)
    pin.set_high();
    // pin.set_low();
    // Timer::after(wf_tail.1).await;

    // // Set back to high (idle)
    // pin.set_high();
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let delay = Duration::from_secs(5);

    // let mut pwm = Pwm::new_output_b(p.PWM_SLICE0, p.PIN_17, c.clone());
    let mut pin = Output::new(p.PIN_16, Level::High);

    pin.set_high();
    // send_command(&mut pin, 0xEF, wf_head, wf_hi, wf_lo, wf_tail).await;
    // send_command(&mut pin, 0xB1, wf_head, wf_hi, wf_lo, wf_tail).await;
    info!("Command sent");
    // Send the command 0x81
    // send_command(&mut pin, 0x81, wf_head, wf_hi, wf_lo, wf_tail).await;
    loop {
        // Send the command 0x81

        info!("Sending command");
        // control.gpio_set(0, true).await;
        send_command(&mut pin, 0xC0).await;

        info!("waiting");
        // control.gpio_set(0, false).await;
        Timer::after(delay).await;
    }
}

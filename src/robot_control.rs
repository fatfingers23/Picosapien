use core::u8;

use crate::commands::RobotCommand;
use defmt::*;
use embassy_rp::gpio::{AnyPin, Level, Output};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

const CYCLE: u64 = 833;

pub struct RobotControl<'d> {
    output_pin: Output<'d>,
}

impl<'d> RobotControl<'d> {
    pub fn new(pin: AnyPin) -> Self {
        let output_pin = Output::new(pin, Level::High);

        Self { output_pin }
    }

    pub async fn send_raw_command(&mut self, command: u8) {
        // Pin is set to high and when low for 8 cycles it signifies a start of a command
        self.output_pin.set_low();
        Timer::after(Duration::from_micros(CYCLE * 8)).await;

        //Convert the command to the 8 bit binary representation
        for i in (0..8).rev() {
            let bit = (command >> i) & 1;
            // Send the high-low sequence based on the bit value
            match bit {
                1 => {
                    self.output_pin.set_high();
                    Timer::after(Duration::from_micros(CYCLE * 4)).await;
                    self.output_pin.set_low();
                    Timer::after(Duration::from_micros(CYCLE * 1)).await;
                }
                0 => {
                    self.output_pin.set_high();
                    Timer::after(Duration::from_micros(CYCLE * 1)).await;
                    self.output_pin.set_low();
                    Timer::after(Duration::from_micros(CYCLE * 1)).await;
                }
                2_u8..=u8::MAX => error!("Invalid bit value"),
            }
        }

        // Set back to high to end the transmission (default)
        self.output_pin.set_high();
    }

    pub async fn _send_command(&mut self, command: RobotCommand) {
        self.send_raw_command(command as u8).await;
    }
}

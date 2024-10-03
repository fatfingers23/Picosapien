#![no_std]
#![no_main]

use commands::RobotCommand;
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    gpio::{Level, Output},
    peripherals::PIO0,
    pio::{InterruptHandler, Pio},
    Peripheral,
};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

mod commands;
mod cyw43_driver;
mod robot_control;

const CYCLE: u64 = 833;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

async fn send_command<'a>(pin: &mut Output<'a>, command: RobotCommand) {
    send_command_raw(pin, command as u8).await;
}

async fn send_command_raw<'a>(pin: &mut Output<'a>, command: u8) {
    // Pin is set to high and when low for 8 cycles it signifies a start of a command
    pin.set_low();
    Timer::after(Duration::from_micros(CYCLE * 8)).await;

    //Convert the command to the 8 bit binary representation
    for i in (0..8).rev() {
        let bit = (command >> i) & 1;
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
    }

    // Set back to high to end the transmission (default)
    pin.set_high();
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let delay = Duration::from_secs(2);
    //This is the pin that the white wire is connected to
    // let mut pin = Output::new(p.PIN_16, Level::High);

    // //The pin is default as high
    // pin.set_high();

    //Bends over and picks up the raspberry pi pico

    let mut pio = Pio::new(p.PIO0, Irqs);

    let mut robot = robot_control::RobotControl::new(&mut pio.common, pio.sm0, p.PIN_16);
    let mut dma_in_ref = p.DMA_CH1.into_ref();

    loop {
        info!("Sending command");

        robot
            .send_command(dma_in_ref.reborrow(), RobotCommand::RightArmUp)
            .await;
        // send_command(&mut pin, RobotCommand::RightArmOut).await;
        Timer::after(delay).await;
    }
}

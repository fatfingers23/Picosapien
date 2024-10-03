#![no_std]
#![no_main]

use commands::RobotCommand;
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::gpio::AnyPin;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

mod commands;
mod cyw43_driver;
mod robot_control;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let delay = Duration::from_secs(2);

    let mut robot = robot_control::RobotControl::new(AnyPin::from(p.PIN_16));

    loop {
        info!("Sending command");

        robot.send_command(RobotCommand::RightArmUp).await;
        // send_command(&mut pin, RobotCommand::RightArmOut).await;
        Timer::after(delay).await;
    }
}

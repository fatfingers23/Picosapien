#![no_std]
#![no_main]

use cyw43::JoinOptions;
use defmt::*;
use embassy_executor::Spawner;
use embassy_net::{Config, StackResources};
use embassy_rp::clocks::RoscRng;
use embassy_time::Timer;
use env::env_value;
use http_server::HttpServer;
use rand::RngCore;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

mod commands;
mod cyw43_driver;
mod env;
mod http_server;
mod robot_control;

use cyw43_driver::{net_task, setup_cyw43};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut rng = RoscRng;

    let (net_device, mut control) = setup_cyw43(
        p.PIO0, p.PIN_23, p.PIN_24, p.PIN_25, p.PIN_29, p.DMA_CH0, spawner,
    )
    .await;

    // Use a link-local address for communication without DHCP server
    // let config = Config::ipv4_static(embassy_net::StaticConfigV4 {
    //     address: embassy_net::Ipv4Cidr::new(embassy_net::Ipv4Address::new(169, 254, 1, 1), 16),
    //     dns_servers: heapless::Vec::new(),
    //     gateway: None,
    // });

    let config = Config::dhcpv4(Default::default());

    // Generate random seed
    let seed = rng.next_u64();

    // Init network stack
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    );

    unwrap!(spawner.spawn(net_task(runner)));
    let ssid = env_value("WIFI_SSID");
    let password = env_value("WIFI_PASSWORD");
    //Open AP
    // control.start_ap_open("Picosapien", 5).await;
    loop {
        //This loop breaks when the join is successful
        match control
            .join(ssid, JoinOptions::new(password.as_bytes()))
            .await
        {
            Ok(_) => break,
            Err(err) => {
                info!("join failed with status={}", err.status);
            }
        }
    }
    // Wait for DHCP, not necessary when using static IP
    info!("waiting for DHCP...");
    while !stack.is_config_up() {
        Timer::after_millis(100).await;
    }
    info!("DHCP is now up!");

    let mut server = HttpServer::new(80, stack);

    server.serve().await;
}

#![no_std]
#![no_main]

use commands::RobotCommand;
use core::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use core::str::from_utf8;
use core::time::Duration as CoreDuration;
use defmt::*;
use edge_nal_embassy::{Udp, UdpBuffers};
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::udp::{UdpMetadata, UdpSocket};
use embassy_net::{Config, Ipv4Address, Stack, StackResources};
use embassy_rp::clocks::RoscRng;
use embassy_time::{Duration, Timer};
use embedded_io_async::Write;
use heapless::Vec;
use rand::RngCore;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

mod commands;
mod cyw43_driver;
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
    let config = Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: embassy_net::Ipv4Cidr::new(embassy_net::Ipv4Address::new(169, 254, 1, 1), 16),
        dns_servers: heapless::Vec::new(),
        gateway: None,
    });

    // Generate random seed
    let seed = rng.next_u64();

    // Init network stack
    static RESOURCES: StaticCell<StackResources<6>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    );

    unwrap!(spawner.spawn(net_task(runner)));

    control.start_ap_open("Picosapien", 5).await;

    // spawner.must_spawn(access_point(stack));

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut buf = [0; 4096];

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(10)));

        control.gpio_set(0, false).await;

        if let Err(e) = socket.accept(80).await {
            warn!("accept error: {:?}", e);
            continue;
        }

        info!("Received connection from {:?}", socket.remote_endpoint());
        control.gpio_set(0, true).await;

        loop {
            let n = match socket.read(&mut buf).await {
                Ok(0) => {
                    warn!("read EOF");
                    break;
                }
                Ok(n) => n,
                Err(e) => {
                    warn!("read error: {:?}", e);
                    break;
                }
            };

            info!("Web request{}", from_utf8(&buf[..n]).unwrap());
            let html = "HTTP/1.0 200 OK\r\nContent-type: text/html\r\n\r\n<!DOCTYPE html>
            <html>
                <body>
                   <h1>Pico W - Hello World!</h1>
                </body>
            </html";

            match socket.write_all(html.as_bytes()).await {
                Ok(()) => {}
                Err(e) => {
                    warn!("write error: {:?}", e);
                    break;
                }
            };
            //Have to close the socket so the web browser knows its done
            socket.close();
        }
    }

    // let delay = Duration::from_secs(2);

    // let mut robot = robot_control::RobotControl::new(AnyPin::from(p.PIN_16));

    // loop {
    //     info!("Sending command");

    //     robot.send_command(RobotCommand::RightArmUp).await;
    //     // send_command(&mut pin, RobotCommand::RightArmOut).await;
    //     Timer::after(delay).await;
    // }
}

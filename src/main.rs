#![no_std]
#![no_main]

use core::borrow::Borrow;
use cyw43::{Control, JoinOptions};
use defmt::*;
use embassy_executor::Spawner;
use embassy_net::{Config, StackResources};
use embassy_rp::clocks::RoscRng;
use embassy_time::Timer;
use env::env_value;
use http_server::{
    HttpServer, Method, Response, StatusCode, WebRequest, WebRequestHandler, WebRequestHandlerError,
};
use io::easy_format_str;
use rand::RngCore;
use reqwless::response::{self};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

mod commands;
mod cyw43_driver;
mod env;
mod http_server;
mod io;
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

    let robot_control = robot_control::RobotControl::new(p.PIN_16.into());

    // Use a link-local address for communication without DHCP server
    let config = Config::ipv4_static(embassy_net::StaticConfigV4 {
        // address: embassy_net::Ipv4Cidr::new(embassy_net::Ipv4Address::new(169, 254, 1, 1), 16),
        address: embassy_net::Ipv4Cidr::new(embassy_net::Ipv4Address::new(192, 168, 68, 53), 16),
        dns_servers: heapless::Vec::new(),
        gateway: Some(embassy_net::Ipv4Address::new(192, 168, 68, 1)),
    });

    // let config = Config::dhcpv4(Default::default());

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

    server
        .serve(WebsiteHandler {
            control,
            robot_control,
        })
        .await;
}

struct WebsiteHandler {
    control: Control<'static>,
    robot_control: robot_control::RobotControl<'static>,
}

impl WebRequestHandler for WebsiteHandler {
    async fn handle_request<'a>(
        &mut self,
        request: WebRequest<'_, '_>,
        response_buffer: &'a mut [u8],
    ) -> Result<Response<'a>, WebRequestHandlerError> {
        if request.path.unwrap().starts_with("/command") {
            let extracted_command = request.path.unwrap().split("/command/").last();
            if extracted_command.is_none() {
                error!("No command found");
                return Ok(Response::new_html(
                    StatusCode::Ok,
                    "No command found in the request",
                ));
            }
            let command = extracted_command.unwrap();
            info!("Command: {:?}", command);
            let parse_command = command.parse::<u8>();
            if parse_command.is_err() {
                error!("Cannot parse command");
                return Ok(Response::new_html(
                    StatusCode::Ok,
                    "Cannot parse command to u8",
                ));
            }
            let command = command.parse::<u8>().unwrap();
            self.robot_control.send_raw_command(command).await;
            return Ok(Response::new_html(StatusCode::Ok, "Command sent"));
        }

        let light_status = match request.path.unwrap() {
            "/" => {
                let web_app = include_str!("../web_app/index.html");
                return Ok(Response::new_html(StatusCode::Ok, web_app));
            }
            "/on" => {
                self.control.gpio_set(0, true).await;
                "on"
            }
            "/off" => {
                self.control.gpio_set(0, false).await;
                "off"
            }
            _ => {
                self.control.gpio_set(0, true).await;
                "Probably off"
            }
        };

        let html_response = easy_format_str(
            format_args!(
                "
            <!DOCTYPE html>
            <html>
                <body>
                    <h1>The light is {light_status}.</h1>
                    
                </body>
            </html>
            "
            ),
            response_buffer,
        );

        Ok(Response::new_html(StatusCode::Ok, html_response.unwrap()))
    }
}

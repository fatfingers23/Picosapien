#![no_std]
#![no_main]

use cyw43::{Control, JoinOptions};
use cyw43_driver::{net_task, setup_cyw43};
use defmt::*;
use embassy_executor::Spawner;
use embassy_net::{Config, StackResources};
use embassy_rp::{clocks::RoscRng, flash::Async, peripherals::FLASH, watchdog::Watchdog};
use embassy_time::{Duration, Timer};
use http_server::{
    HttpServer, Response, StatusCode, WebRequest, WebRequestHandler, WebRequestHandlerError,
};
use io::easy_format_str;
use rand::RngCore;
use save::{erase_save_flash, read_postcard_from_flash, save_postcard_to_flash, Save};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

mod commands;
mod cyw43_driver;
mod env;
mod http_server;
mod io;
mod robot_control;
mod save;

const FLASH_SIZE: usize = 2 * 1024 * 1024;

#[embassy_executor::task]
async fn watchdog_task(mut watchdog: Watchdog) {
    loop {
        watchdog.feed();
        Timer::after(Duration::from_secs(4)).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut watchdog = Watchdog::new(p.WATCHDOG);
    let mut rng = RoscRng;
    let mut flash: embassy_rp::flash::Flash<'static, FLASH, Async, FLASH_SIZE> =
        embassy_rp::flash::Flash::<'static, FLASH, Async, FLASH_SIZE>::new(p.FLASH, p.DMA_CH3);
    // Generate random seed
    let seed = rng.next_u64();

    let (net_device, mut control) = setup_cyw43(
        p.PIO0, p.PIN_23, p.PIN_24, p.PIN_25, p.PIN_29, p.DMA_CH0, spawner,
    )
    .await;

    let robot_control = robot_control::RobotControl::new(p.PIN_16.into());

    let mut turn_on_ap = false;

    //TODO IF a save is found join that network, if not open AP
    //Also look at if it fails to start ap
    //Also look at having a watchdog timer to restart the device if it fails to connect to a network

    let join_another_net_work_config = Config::dhcpv4(Default::default());

    // Init network stack
    static RESOURCES: StaticCell<StackResources<4>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        join_another_net_work_config,
        RESOURCES.init(StackResources::new()),
        seed,
    );
    spawner.must_spawn(net_task(runner));

    control.gpio_set(0, true).await;

    let request_to_read_flash = read_postcard_from_flash(&mut flash);
    match request_to_read_flash {
        Ok(mut save) => {
            watchdog.start(Duration::from_secs(8));

            //If the last save says clear on boot. Clear and restart
            if save.clear_on_boot {
                info!("Clear on boot flag set. Clearing flash and restarting");
                erase_save_flash(&mut flash);
                watchdog.trigger_reset();
            }
            //Sets a clear on boot flag before attempting to connect so we know if its never cleared we need to restart with a clear flash
            save.clear_on_boot = true;
            let _ = save_postcard_to_flash(&mut flash, &save);
            let mut wifi_connection_attempts = 0;
            let mut was_able_to_connect = false;
            while wifi_connection_attempts < 30 {
                debug!("Attempting to connect to wifi: {}", save.wifi_ssid);
                let attempt_to_connect = control
                    .join(
                        save.wifi_ssid.as_str(),
                        JoinOptions::new(save.wifi_password.as_bytes()),
                    )
                    .await;
                match attempt_to_connect {
                    Ok(_) => {
                        info!("join successful");
                        was_able_to_connect = true;
                        break;
                    }
                    Err(err) => {
                        error!("join failed with status={}", err.status);
                    }
                }
                Timer::after(Duration::from_secs(1)).await;
                wifi_connection_attempts += 1;
            }
            if was_able_to_connect {
                save.clear_on_boot = false;
                let _ = save_postcard_to_flash(&mut flash, &save);
            } else {
                turn_on_ap = true;
            }
            //Spawn watch dog task
            spawner.must_spawn(watchdog_task(watchdog));
        }
        Err(err) => {
            error!("Error reading flash: {:?}", err);
            turn_on_ap = true;
        }
    }

    if turn_on_ap {
        info!("Could not connect to save connection bringing up AP");
        // Use a link-local address for communication without DHCP server
        stack.set_config_v4(embassy_net::ConfigV4::Static(embassy_net::StaticConfigV4 {
            address: embassy_net::Ipv4Cidr::new(embassy_net::Ipv4Address::new(169, 254, 1, 1), 16),
            dns_servers: heapless::Vec::new(),
            gateway: None,
        }));
        control.start_ap_open("Picosapien", 5).await;
    }
    info!("waiting for DHCP...");
    while !stack.is_config_up() {
        Timer::after_millis(100).await;
    }
    info!("DHCP is now up!");

    let mut server = HttpServer::new(80, stack);

    server
        .serve(WebsiteHandler {
            control,
            flash,
            robot_control,
        })
        .await;
}

struct WebsiteHandler {
    control: Control<'static>,
    flash: embassy_rp::flash::Flash<'static, FLASH, Async, FLASH_SIZE>,
    // flash: FLASH,
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
            "/wifi" => {
                let wifi_page = include_str!("../web_app/wifi.html");
                return Ok(Response::new_html(StatusCode::Ok, wifi_page));
            }
            "/SaveWifi" => {
                let result = serde_json_core::from_str::<Save>(request.body);

                if result.is_err() {
                    return Ok(Response::new_html(
                        StatusCode::BadRequest,
                        "Error parsing json from request",
                    ));
                }

                let (save, _) = result.unwrap();

                let save_result = save_postcard_to_flash(&mut self.flash, &save);
                if save_result.is_err() {
                    return Ok(Response::new_html(
                        StatusCode::InternalServerError,
                        "Error saving wifi credentials to flash",
                    ));
                }
                return Ok(Response::new_html(StatusCode::Ok, "Wifi has been saved"));
            }
            "/on" => {
                self.control.gpio_set(0, true).await;
                "on"
            }
            "/off" => {
                self.control.gpio_set(0, false).await;
                "off"
            }
            _ => "Probably off",
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

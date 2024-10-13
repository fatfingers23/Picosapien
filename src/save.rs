use crate::FLASH_SIZE;
use defmt::*;
use embassy_rp::flash::{Async, ERASE_SIZE};
use embassy_rp::peripherals::FLASH;
use heapless::String;
use postcard::{from_bytes, to_slice};
use serde::{Deserialize, Serialize};
use {defmt_rtt as _, panic_probe as _};

const ADDR_OFFSET: u32 = 0x100000;
const SAVE_OFFSET: u32 = 0x00;

pub fn save_postcard_to_flash(
    flash: &mut embassy_rp::flash::Flash<'_, FLASH, Async, FLASH_SIZE>,
    data: &Save,
) -> Result<(), &'static str> {
    let mut write_buf = [0u8; ERASE_SIZE];
    let written = to_slice(data, &mut write_buf).map_err(|_| "Serialization error")?;

    if written.len() > ERASE_SIZE {
        return Err("Data too large for flash sector");
    }

    erase_save_flash(flash);

    // buf[..written.len()].copy_from_slice(&written);
    let save_as_str = core::str::from_utf8(&written);
    if save_as_str.is_ok() {
        info!("saving as str: {:?}", save_as_str.unwrap());
    }
    flash
        .blocking_write(ADDR_OFFSET + SAVE_OFFSET, &written)
        .map_err(|_| "Write error")?;

    Ok(())
}

pub fn read_postcard_from_flash(
    flash: &mut embassy_rp::flash::Flash<'_, FLASH, Async, FLASH_SIZE>,
) -> Result<Save, &'static str> {
    let mut buf = [0u8; ERASE_SIZE];

    let result = flash
        .blocking_read(ADDR_OFFSET + SAVE_OFFSET, &mut buf)
        .map_err(|e| e);
    if result.is_err() {
        info!("Error reading flash: {:?}", result.err());
    }

    let save_as_str = core::str::from_utf8(&buf);
    if save_as_str.is_ok() {
        info!("Reading as str: {:?}", save_as_str.unwrap());
    }
    let data = from_bytes::<Save>(&buf);
    match data {
        Ok(data) => {
            debug!("Save Data: {:?}", data);
            return Ok(data);
        }
        Err(e) => {
            //This should mean no data has been saved
            error!("Error deserializing: {:?}", e);
            return Err("Error deserializing");
        }
    }
}

pub fn erase_save_flash(flash: &mut embassy_rp::flash::Flash<'_, FLASH, Async, FLASH_SIZE>) {
    debug!("Erasing save flash");
    flash
        .blocking_erase(
            ADDR_OFFSET + SAVE_OFFSET,
            ADDR_OFFSET + SAVE_OFFSET + ERASE_SIZE as u32,
        )
        .unwrap();
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, defmt::Format)]
pub struct Save {
    pub clear_on_boot: bool,
    pub wifi_ssid: String<32>,
    pub wifi_password: String<32>,
}

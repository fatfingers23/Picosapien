use crate::FLASH_SIZE;
use defmt::info;
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
    let mut buf = [0u8; ERASE_SIZE];

    let mut write_buf = [0u8; ERASE_SIZE];
    let written = to_slice(data, &mut write_buf).map_err(|_| "Serialization error")?;

    if written.len() > ERASE_SIZE {
        return Err("Data too large for flash sector");
    }

    flash
        .blocking_erase(
            ADDR_OFFSET + SAVE_OFFSET,
            ADDR_OFFSET + SAVE_OFFSET + ERASE_SIZE as u32,
        )
        .map_err(|_| "Erase error")?;

    buf[..written.len()].copy_from_slice(&written);

    flash
        .blocking_write(ADDR_OFFSET + SAVE_OFFSET, &buf)
        .map_err(|_| "Write error")?;

    Ok(())
}

pub fn read_postcard_from_flash(
    flash: &mut embassy_rp::flash::Flash<'_, FLASH, Async, FLASH_SIZE>,
) -> Result<Option<Save>, &'static str> {
    let mut buf = [0u8; ERASE_SIZE];

    let result = flash
        .blocking_read(ADDR_OFFSET + SAVE_OFFSET, &mut buf)
        .map_err(|e| e);
    if result.is_err() {
        info!("Error reading flash: {:?}", result.err());
    }
    //get result length ignoring trailing zeros
    let len = buf.iter().position(|&r| r == 0).unwrap_or(buf.len());
    let buf: &[u8] = &buf[..len];
    let data_as_str = core::str::from_utf8(&buf).map_err(|_| "Invalid UTF-8")?;
    info!("Data as str: {:?}", data_as_str);

    let data = from_bytes::<Option<Save>>(&buf).map_err(|_| "Deserialization error")?;
    Ok(data)
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, defmt::Format)]
pub struct Save {
    pub wifi_ssid: String<32>,
    pub wifi_password: String<32>,
}

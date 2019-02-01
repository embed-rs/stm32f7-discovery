//! Touchscreen functions.

use crate::i2c::{self, I2C};
use arrayvec::ArrayVec;
use stm32f7::stm32f7x6 as device;

const FT5336_ADDRESS: i2c::Address = i2c::Address::bits_7(0b0111000);
const FT5336_FAMILY_ID_REGISTER: u8 = 0xA8;
const FT5336_STATUS_REGISTER: u8 = 0x02;

// Start locations for reading pressed touches
const FT5336_DATA_REGISTERS: [u8; 5] = [0x03, 0x09, 0x0F, 0x15, 0x1B];

/// Checks the whether the device familiy ID register contains the expected value.
pub fn check_family_id(i2c_3: &mut I2C<device::I2C3>) -> Result<(), i2c::Error> {
    i2c_3.connect::<u8, _>(FT5336_ADDRESS, |mut conn| {
        // read and check device family ID
        assert_eq!(conn.read(FT5336_FAMILY_ID_REGISTER).ok(), Some(0x51));
        Ok(())
    })
}

#[derive(Debug, Clone, Copy)]
/// Represents a touch point on the display at coordinates (x,y).
pub struct Touch {
    /// The x coordinate of the touch point (horizontal).
    pub x: u16,
    /// The y coordinate of the touch point (vertical).
    pub y: u16,
}

/// Returns a list of active touch points.
pub fn touches(i2c_3: &mut I2C<device::I2C3>) -> Result<ArrayVec<[Touch; 5]>, i2c::Error> {
    let mut touches = ArrayVec::new();
    i2c_3.connect::<u8, _>(FT5336_ADDRESS, |mut conn| {
        let status = conn.read(FT5336_STATUS_REGISTER)?;
        let mut number_of_touches = status & 0x0F;
        if number_of_touches > 5 {
            number_of_touches = 0;
        }

        for &data_reg in FT5336_DATA_REGISTERS.iter().take(number_of_touches.into()) {
            let mut touch_data: [u8; 4] = [0; 4];
            conn.read_bytes(data_reg, &mut touch_data)?;
            let y = (u16::from(touch_data[0] & 0x0F) << 8) | u16::from(touch_data[1]);
            let x = (u16::from(touch_data[2] & 0x0F) << 8) | u16::from(touch_data[3]);
            touches.push(Touch { x: x, y: y });
        }
        Ok(())
    })?;

    Ok(touches)
}

use arrayvec::ArrayVec;
use i2c::{self, I2C};

const FT5336_ADDRESS: i2c::Address = i2c::Address::bits_7(0b0111000);
const FT5336_FAMILY_ID_REGISTER: u8 = 0xA8;
const FT5336_STATUS_REGISTER: u8 = 0x02;

// Start locations for reading pressed touches
const FT5336_DATA_REGISTERS: [u8; 5] = [0x03, 0x09, 0x0F, 0x15, 0x1B];

pub fn check_family_id(i2c_3: &mut I2C) -> Result<(), i2c::Error> {
    i2c_3.connect::<u8, _>(FT5336_ADDRESS, |mut conn| {
        // read and check device family ID
        // FIXME: This assertion fails in release mode (read returns Err(_))
        assert_eq!(conn.read(FT5336_FAMILY_ID_REGISTER).ok(), Some(0x51));
        Ok(())
    })
}

#[derive(Debug, Clone, Copy)]
pub struct Touch {
    pub x: u16,
    pub y: u16,
}

pub fn touches(i2c_3: &mut I2C) -> Result<ArrayVec<[Touch; 5]>, i2c::Error> {
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

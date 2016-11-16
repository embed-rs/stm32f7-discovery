/// https://cdn-shop.adafruit.com/datasheets/FT6x06_AN_public_ver0.1.3.pdf

use i2c;

const FT6X06_ADDRESS: i2c::Address = i2c::Address::U7(0b0111000);

pub fn init_ft6x06(i2c_3: &mut i2c::I2C) -> Result<(), i2c::Error> {
    // reset device
    i2c_3.write(FT6X06_ADDRESS, 0, 0)?;
    // read and check device family ID
    assert_eq!(i2c_3.read(FT6X06_ADDRESS, 0xA8)?, 0x11);
    // set polling mode
    i2c_3.write(FT6X06_ADDRESS, 0xA4, 0)?;
    Ok(())
}

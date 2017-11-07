pub use self::init::init;

pub mod error;
mod init;
mod command;

use board::sdmmc::Sdmmc;

pub struct Sd {
    _sdmmc: &'static mut Sdmmc,
    card_type: CardType,
    rca: u16,
}

impl Sd {
    pub fn get_card_type(&self) -> &CardType {
        &self.card_type
    }
    pub fn get_rca(&self) -> u16 {
        self.rca
    }
}

#[derive(Debug)]
pub enum CardType {
    SDv1,
    SDv2SC,
    SDv2HC,
}

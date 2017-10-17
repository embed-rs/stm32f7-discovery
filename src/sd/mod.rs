pub use self::init::init;

pub mod error;
mod init;
mod command;

use board::sdmmc::Sdmmc;

pub struct Sd {
    _registers: &'static mut Sdmmc,
    _card_type: CardType,
}

#[allow(dead_code)]
enum CardType {
    SDv1,
    SDv2,
}


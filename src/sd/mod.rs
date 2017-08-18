pub use self::init::init;

mod init;
mod command;

use board::sdmmc::Sdmmc;

#[derive(Debug)]
pub enum Error {
    NoSdCard,
    Timeout,
}

pub struct Sd {
    _registers: &'static mut Sdmmc,
    _card_type: CardType,
}

#[allow(dead_code)]
enum CardType {
    Mmc,
    Sd,
    Sdio,
}


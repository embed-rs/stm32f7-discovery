use super::error::*;
use stm32f7::stm32f7x6::SDMMC1;

// Initialization commands
/// Set the SD card into idle state
pub fn idle(sdmmc: &mut SDMMC1, timeout: u32) -> Result<(), Error> {
    send_cmd(sdmmc, 0, 0x00, true, false, 0x00);

    let timeout = crate::system_clock::ms() as u32 + timeout;
    while (crate::system_clock::ms() as u32) < timeout && sdmmc.sta.read().cmdsent().bit_is_clear()
    {
    }

    if (crate::system_clock::ms() as u32) >= timeout {
        return Err(Error::Timeout);
    }

    Ok(())
}

/// Send CMD55 to signalize that the next command is an app command
pub fn app(sdmmc: &mut SDMMC1, argument: u32) -> Result<(), Error> {
    send_cmd(sdmmc, argument, 55, true, false, 0x01);

    get_cmd_resp1(sdmmc, 55, 5000)
}

/// Send ACMD41 to get the operation condition register (OCR) of the card.
/// Always send CMD55 before sending this command.
pub fn app_oper(sdmmc: &mut SDMMC1, capacity: u32) -> Result<(), Error> {
    send_cmd(sdmmc, 0x8010_0000 | capacity, 41, true, false, 0x01);

    get_cmd_resp3(sdmmc, 5000)
}

/// Get the Operation Condition of the card. This command is only supported
/// by SD card v2 and can therefore be used to determine the version of the card.
pub fn oper_cond(sdmmc: &mut SDMMC1) -> Result<(), Error> {
    send_cmd(sdmmc, 0x1AA, 8, true, false, 0x01);

    wait_resp(sdmmc, 5000)?;

    sdmmc.icr.modify(|_, w| w.cmdrendc().set_bit());

    Ok(())
}

/// Get the Card Indentification Number (CID) of the card. (CMD2)
pub fn send_cid(sdmmc: &mut SDMMC1) -> Result<(), Error> {
    send_cmd(sdmmc, 0, 2, true, false, 0x03);

    get_cmd_resp2(sdmmc, 5000)
}

/// Get the Relative Card Address (RCA) of the card. This number is shorter
/// than the CID. (CMD3)
pub fn set_rel_add(sdmmc: &mut SDMMC1) -> Result<u16, Error> {
    send_cmd(sdmmc, 0, 3, true, false, 0x01);

    get_cmd_resp6(sdmmc, 3, 5000)
}

pub fn send_csd(sdmmc: &mut SDMMC1, rca: u32) -> Result<(), Error> {
    send_cmd(sdmmc, rca, 9, true, false, 0x03);

    get_cmd_resp2(sdmmc, 5000)
}

pub fn sel_desel(sdmmc: &mut SDMMC1, rca: u32) -> Result<(), Error> {
    send_cmd(sdmmc, rca, 7, true, false, 0x01);

    get_cmd_resp1(sdmmc, 7, 5000)
}

// Read/Write commands
/// Set the block length of the blocks to read/write.
pub fn block_length(sdmmc: &mut SDMMC1, block_size: u32) -> Result<(), Error> {
    send_cmd(sdmmc, block_size, 16, true, false, 0x01);

    get_cmd_resp1(sdmmc, 16, 5000)
}

/// Instruct the controller, that a single block will be written.
pub fn write_single_blk(sdmmc: &mut SDMMC1, block_add: u32) -> Result<(), Error> {
    send_cmd(sdmmc, block_add, 24, true, false, 0x01);

    get_cmd_resp1(sdmmc, 24, 5000)
}

/// Instruct the controller, that multiple blocks will be written. End the write process with a
/// call to `stop_transfer()`.
// TODO: This doesn't seem to work...
pub fn write_multi_blk(sdmmc: &mut SDMMC1, block_add: u32) -> Result<(), Error> {
    send_cmd(sdmmc, block_add, 25, true, false, 0x01);

    get_cmd_resp1(sdmmc, 25, 5000)
}

/// Instruct the controller, that a single block will be read.
pub fn read_single_blk(sdmmc: &mut SDMMC1, block_add: u32) -> Result<(), Error> {
    send_cmd(sdmmc, block_add, 17, true, false, 0x01);

    get_cmd_resp1(sdmmc, 17, 5000)
}

/// Instruct the controller, that multiple blocks will be read. End the read process with a
/// call to `stop_transfer()`.
// TODO: This doesn't seem to work...
pub fn read_multi_blk(sdmmc: &mut SDMMC1, block_add: u32) -> Result<(), Error> {
    send_cmd(sdmmc, block_add, 18, true, false, 0x01);

    get_cmd_resp1(sdmmc, 18, 5000)
}

// An alternative, to end multi-block read/write with `stop_transfer()`, is to specify the number of
// blocks that should be written beforehand.
// The controller doesn't seem to accept this command and always returns with a CmdRespTimeout Error.
// pub fn set_blk_count(sdmmc: &mut SDMMC1, number_of_blks: u16) -> Result<(), Error> {
//     send_cmd(sdmmc, number_of_blks as u32, 23, true, false, 0x01);
//
//     get_cmd_resp1(sdmmc, 23, 5000)
// }

/// Stops the tranfer to the card after a multi-block read/write.
pub fn stop_transfer(sdmmc: &mut SDMMC1) -> Result<(), Error> {
    send_cmd(sdmmc, 0, 12, true, false, 0x01);

    get_cmd_resp1(sdmmc, 12, 5000)?;

    Ok(())
}

/// Send a command to the card.
pub fn send_cmd(
    sdmmc: &mut SDMMC1,
    argument: u32,
    cmdidx: u8,
    cpsmen: bool,
    waitint: bool,
    waitresp: u8,
) {
    sdmmc
        .arg
        .modify(|_, w| unsafe { w.cmdarg().bits(argument) });
    sdmmc.cmd.modify(|_, w| {
        w.cpsmen().bit(cpsmen);
        w.waitint().bit(waitint);
        unsafe {
            w.waitresp().bits(waitresp);
            w.cmdindex().bits(cmdidx);
        }
        w
    });
}

// Command responses from the controller
fn get_cmd_resp1(sdmmc: &mut SDMMC1, cmd_idx: u8, timeout: u32) -> Result<(), Error> {
    wait_resp_crc(sdmmc, timeout)?;

    if sdmmc.respcmd.read().respcmd().bits() != cmd_idx {
        return Err(Error::SdmmcError {
            t: SdmmcErrorType::CmdCrcFailed,
        });
    }

    clear_all_static_status_flags(sdmmc);

    // Get response and check card status for errors
    let card_status = sdmmc.resp1.read().cardstatus1().bits();

    check_for_errors(card_status)?;

    Ok(())
}

fn get_cmd_resp2(sdmmc: &mut SDMMC1, timeout: u32) -> Result<(), Error> {
    wait_resp_crc(sdmmc, timeout)?;

    clear_all_static_status_flags(sdmmc);

    Ok(())
}

fn get_cmd_resp3(sdmmc: &mut SDMMC1, timeout: u32) -> Result<(), Error> {
    wait_resp(sdmmc, timeout)?;

    clear_all_static_status_flags(sdmmc);

    Ok(())
}

fn get_cmd_resp6(sdmmc: &mut SDMMC1, cmd_idx: u8, timeout: u32) -> Result<u16, Error> {
    use super::error::CardStatusFlags;

    wait_resp_crc(sdmmc, timeout)?;

    if sdmmc.respcmd.read().respcmd().bits() != cmd_idx {
        return Err(Error::SdmmcError {
            t: SdmmcErrorType::CmdCrcFailed,
        });
    }

    clear_all_static_status_flags(sdmmc);

    // Get response and check card status for errors
    let card_status = sdmmc.resp1.read().cardstatus1().bits();

    if card_status
        & (CardStatusFlags::R6_CRC_FAILED
            | CardStatusFlags::R6_ILLEGAL_COMMAND
            | CardStatusFlags::R6_GENERAL_UNKNOWN_ERROR)
            .bits()
        == 0
    {
        Ok((card_status >> 16) as u16)
    } else if card_status & CardStatusFlags::R6_CRC_FAILED.bits() != 0 {
        Err(Error::CardError {
            t: CardStatusFlags::R6_CRC_FAILED,
        })
    } else if card_status & CardStatusFlags::R6_ILLEGAL_COMMAND.bits() != 0 {
        Err(Error::CardError {
            t: CardStatusFlags::R6_ILLEGAL_COMMAND,
        })
    } else {
        Err(Error::CardError {
            t: CardStatusFlags::R6_GENERAL_UNKNOWN_ERROR,
        })
    }
}

// Wait for the Controller to respond to a command.
fn wait_resp(sdmmc: &mut SDMMC1, timeout: u32) -> Result<(), Error> {
    let timeout = crate::system_clock::ms() as u32 + timeout;
    while (crate::system_clock::ms() as u32) < timeout
        && sdmmc.sta.read().cmdrend().bit_is_clear()
        && sdmmc.sta.read().ccrcfail().bit_is_clear()
        && sdmmc.sta.read().ctimeout().bit_is_clear()
    {}

    if (crate::system_clock::ms() as u32) >= timeout {
        return Err(Error::Timeout);
    }

    if sdmmc.sta.read().ctimeout().bit_is_set() {
        sdmmc.icr.modify(|_, w| w.ctimeoutc().set_bit());
        return Err(Error::SdmmcError {
            t: SdmmcErrorType::CmdRespTimeout,
        });
    }

    Ok(())
}

// Similiar to wait_resp(), but also checks the CRC afterwards
fn wait_resp_crc(sdmmc: &mut SDMMC1, timeout: u32) -> Result<(), Error> {
    wait_resp(sdmmc, timeout)?;
    if sdmmc.sta.read().ccrcfail().bit_is_set() {
        sdmmc.icr.modify(|_, w| w.ccrcfailc().set_bit());
        return Err(Error::SdmmcError {
            t: SdmmcErrorType::CmdCrcFailed,
        });
    }

    Ok(())
}

pub fn clear_all_static_status_flags(sdmmc: &mut SDMMC1) {
    sdmmc.icr.modify(|_, w| {
        w.ccrcfailc().set_bit();
        w.dcrcfailc().set_bit();
        w.ctimeoutc().set_bit();
        w.dtimeoutc().set_bit();
        w.txunderrc().set_bit();
        w.rxoverrc().set_bit();
        w.cmdrendc().set_bit();
        w.cmdsentc().set_bit();
        w.dataendc().set_bit();
        w.dbckendc().set_bit();
        w
    });
}

fn check_for_errors(card_status: u32) -> Result<(), Error> {
    use super::error::CardStatusFlags;

    if card_status & CardStatusFlags::OCR_ERROR_BITS.bits() == 0 {
        Ok(())
    } else if card_status & CardStatusFlags::AKE_SEQ_ERROR.bits() != 0 {
        Err(Error::CardError {
            t: CardStatusFlags::AKE_SEQ_ERROR,
        })
    } else if card_status & CardStatusFlags::ERASE_RESET.bits() != 0 {
        Err(Error::CardError {
            t: CardStatusFlags::ERASE_RESET,
        })
    } else if card_status & CardStatusFlags::CARD_ECC_DISABLED.bits() != 0 {
        Err(Error::CardError {
            t: CardStatusFlags::CARD_ECC_DISABLED,
        })
    } else if card_status & CardStatusFlags::WP_ERASE_SKIP.bits() != 0 {
        Err(Error::CardError {
            t: CardStatusFlags::WP_ERASE_SKIP,
        })
    } else if card_status & CardStatusFlags::CID_CSD_OVERWRITE.bits() != 0 {
        Err(Error::CardError {
            t: CardStatusFlags::CID_CSD_OVERWRITE,
        })
    } else if card_status & CardStatusFlags::CC_ERROR.bits() != 0 {
        Err(Error::CardError {
            t: CardStatusFlags::CC_ERROR,
        })
    } else if card_status & CardStatusFlags::CARD_ECC_FAILED.bits() != 0 {
        Err(Error::CardError {
            t: CardStatusFlags::CARD_ECC_FAILED,
        })
    } else if card_status & CardStatusFlags::ILLEGAL_COMMAND.bits() != 0 {
        Err(Error::CardError {
            t: CardStatusFlags::ILLEGAL_COMMAND,
        })
    } else if card_status & CardStatusFlags::COM_CRC_ERROR.bits() != 0 {
        Err(Error::CardError {
            t: CardStatusFlags::COM_CRC_ERROR,
        })
    } else if card_status & CardStatusFlags::LOCK_UNLOCK_FAILED.bits() != 0 {
        Err(Error::CardError {
            t: CardStatusFlags::LOCK_UNLOCK_FAILED,
        })
    } else if card_status & CardStatusFlags::WP_VIOLATION.bits() != 0 {
        Err(Error::CardError {
            t: CardStatusFlags::WP_VIOLATION,
        })
    } else if card_status & CardStatusFlags::ERASE_PARAM.bits() != 0 {
        Err(Error::CardError {
            t: CardStatusFlags::ERASE_PARAM,
        })
    } else if card_status & CardStatusFlags::ERASE_SEQ_ERROR.bits() != 0 {
        Err(Error::CardError {
            t: CardStatusFlags::ERASE_SEQ_ERROR,
        })
    } else if card_status & CardStatusFlags::BLOCK_LEN_ERROR.bits() != 0 {
        Err(Error::CardError {
            t: CardStatusFlags::BLOCK_LEN_ERROR,
        })
    } else if card_status & CardStatusFlags::ADDRESS_MISALIGNED.bits() != 0 {
        Err(Error::CardError {
            t: CardStatusFlags::ADDRESS_MISALIGNED,
        })
    } else if card_status & CardStatusFlags::ADDRESS_OUT_OF_RANGE.bits() != 0 {
        Err(Error::CardError {
            t: CardStatusFlags::ADDRESS_OUT_OF_RANGE,
        })
    } else {
        Err(Error::CardError {
            t: CardStatusFlags::ERROR,
        })
    }
}

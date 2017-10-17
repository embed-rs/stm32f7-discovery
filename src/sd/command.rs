use super::error::*;
use board::sdmmc::Sdmmc;

pub fn send_command_idle(sdmmc: &mut Sdmmc, timeout: u32) -> Result<(), Error> {
    send_command(sdmmc, 0, 0x00, true, false, false, 0x03, false);

    let mut timeout = ::system_clock::ticks() as u32 + timeout;
    while (::system_clock::ticks() as u32) < timeout
        && !sdmmc.sta.read().cmdsent() { timeout -=1; }

    if timeout == 0 {
        return Err(Error::Timeout);
    }

    Ok(())
}

pub fn send_command_app(sdmmc: &mut Sdmmc, argument: u32) -> Result<u32, Error> {
    send_command(sdmmc, argument, 55, true, false, false, 0x01, false);

    get_cmd_resp1(sdmmc, 55, 5000)
}

pub fn send_command_oper_cond(sdmmc: &mut Sdmmc) -> Result<(), Error> {
    send_command(sdmmc, 0x1AA, 8, true, false, false, 0x01, false);

    wait_resp(sdmmc, 5000)?;

    sdmmc.icr.update(|icr| icr.set_cmdrendc(true));

    Ok(())
}

pub fn send_command(sdmmc: &mut Sdmmc,
                argument: u32, cmdidx: u8,
                cpsmen: bool,
                waitpend: bool, waitint: bool, waitresp: u8,
                stdiosus: bool) {
    sdmmc.arg.update(|arg| arg.set_cmdarg(argument));
    sdmmc.cmd.update(|cmd| {
        cmd.set_sdiosuspend(stdiosus);
        cmd.set_cpsmen(cpsmen);
        cmd.set_waitpend(waitpend);
        cmd.set_waitint(waitint);
        cmd.set_waitresp(waitresp);
        cmd.set_cmdindex(cmdidx);
    });
}

fn get_cmd_resp1(sdmmc: &mut Sdmmc, cmd_idx: u8, timeout: u32) -> Result<u32, Error> {
    wait_resp_crc(sdmmc, timeout)?;

    if sdmmc.respcmd.read().respcmd() != cmd_idx {
        return Err(Error::SdmmcError {
            t: SdmmcErrorType::CmdCrcFailed
        });
    }

    clear_all_static_status_flags(sdmmc);

    // Get response and check card status for errors
    let card_status = sdmmc.resp1.read().cardstatus1();

    check_for_errors(card_status)?;

    Ok(card_status)
}

fn get_cmd_resp2(sdmmc: &mut Sdmmc, timeout: u32) -> Result<(), Error> {
    wait_resp_crc(sdmmc, timeout)?;

    clear_all_static_status_flags(sdmmc);

    Ok(())
}

fn get_cmd_resp3(sdmmc: &mut Sdmmc, timeout: u32) -> Result<(), Error> {
    wait_resp(sdmmc, timeout)?;

    clear_all_static_status_flags(sdmmc);

    Ok(())
}

fn wait_resp(sdmmc: &mut Sdmmc, timeout: u32) -> Result<(), Error> {
    let mut timeout = ::system_clock::ticks() as u32 + timeout;
    while (::system_clock::ticks() as u32) < timeout
        && !sdmmc.sta.read().cmdrend()
        && !sdmmc.sta.read().ccrcfail()
        && !sdmmc.sta.read().ctimeout() { timeout -=1; }

    if timeout == 0 {
        return Err(Error::Timeout);
    }

    if sdmmc.sta.read().ctimeout() {
        sdmmc.icr.update(|icr| icr.set_ctimeoutc(true));
        return Err(Error::SdmmcError {
            t: SdmmcErrorType::CmdRespTimeout
        });
    }

    Ok(())
}

fn wait_resp_crc(sdmmc: &mut Sdmmc, timeout: u32) -> Result<(), Error> {
    wait_resp(sdmmc, timeout)?;
    if sdmmc.sta.read().ccrcfail() {
        sdmmc.icr.update(|icr| icr.set_ccrcfailc(true));
        return Err(Error::SdmmcError {
            t: SdmmcErrorType::CmdCrcFailed
        });
    }

    Ok(())
}

//TODO:move?
fn clear_all_static_status_flags(sdmmc: &mut Sdmmc) {
    sdmmc.icr.update(|icr| {
        icr.set_ccrcfailc(true);
        icr.set_dcrcfailc(true);
        icr.set_ctimeoutc(true);
        icr.set_dtimeoutc(true);
        icr.set_txunderrc(true);
        icr.set_rxoverrc(true);
        icr.set_cmdrendc(true);
        icr.set_cmdsentc(true);
        icr.set_dataendc(true);
        icr.set_dbckendc(true);
    });
}

fn check_for_errors(card_status: u32) -> Result<(), Error> {
    if card_status & OCR_ERROR_BITS.bits() == 0 {
        return Ok(())
    } else if card_status & AKE_SEQ_ERROR.bits() != 0 {
        return Err(Error::CardError { t: AKE_SEQ_ERROR });
    } else if card_status & ERASE_RESET.bits() != 0 {
        return Err(Error::CardError { t: ERASE_RESET });
    } else if card_status & CARD_ECC_DISABLED.bits() != 0 {
        return Err(Error::CardError { t: CARD_ECC_DISABLED });
    } else if card_status & WP_ERASE_SKIP.bits() != 0 {
        return Err(Error::CardError { t: WP_ERASE_SKIP });
    } else if card_status & CID_CSD_OVERWRITE.bits() != 0 {
        return Err(Error::CardError { t: CID_CSD_OVERWRITE });
    } else if card_status & CC_ERROR.bits() != 0 {
        return Err(Error::CardError { t: CC_ERROR });
    } else if card_status & CARD_ECC_FAILED.bits() != 0 {
        return Err(Error::CardError { t: CARD_ECC_FAILED });
    } else if card_status & ILLEGAL_COMMAND.bits() != 0 {
        return Err(Error::CardError { t: ILLEGAL_COMMAND });
    } else if card_status & COM_CRC_ERROR.bits() != 0 {
        return Err(Error::CardError { t: COM_CRC_ERROR });
    } else if card_status & LOCK_UNLOCK_FAILED.bits() != 0 {
        return Err(Error::CardError { t: LOCK_UNLOCK_FAILED });
    } else if card_status & WP_VIOLATION.bits() != 0 {
        return Err(Error::CardError { t: WP_VIOLATION });
    } else if card_status & ERASE_PARAM.bits() != 0 {
        return Err(Error::CardError { t: ERASE_PARAM });
    } else if card_status & ERASE_SEQ_ERROR.bits() != 0 {
        return Err(Error::CardError { t: ERASE_SEQ_ERROR });
    } else if card_status & AKE_SEQ_ERROR.bits() != 0 {
        return Err(Error::CardError { t: AKE_SEQ_ERROR });
    } else if card_status & BLOCK_LEN_ERROR.bits() != 0 {
        return Err(Error::CardError { t: BLOCK_LEN_ERROR });
    } else if card_status & ADDRESS_MISALIGNED.bits() != 0 {
        return Err(Error::CardError { t: ADDRESS_MISALIGNED });
    } else if card_status & ADDRESS_OUT_OF_RANGE.bits() != 0 {
        return Err(Error::CardError { t: ADDRESS_OUT_OF_RANGE });
    } else {
        return Err(Error::CardError { t: ERROR });
    }
}

use super::Error;
use board::sdmmc::Sdmmc;

pub fn send_command_idle(sdmmc: &mut Sdmmc, mut timeout: u32) -> Result<(), Error> {
    send_command(sdmmc, 0, 0x00, true, false, false, 0x03, false);

    while timeout > 0
        && !sdmmc.sta.read().cmdsent() { timeout -=1; }

    if timeout == 0 {
        return Err(Error::Timeout);
    }

    Ok(())
}

pub fn send_command_app(sdmmc: &mut Sdmmc, argument: u32) -> Result<(), Error> {
    send_command(sdmmc, argument, 55, true, false, false, 0x01, false);

    Ok(())
}

pub fn send_command_oper_cond(sdmmc: &mut Sdmmc) -> Result<(), Error> {
    send_command(sdmmc, 0x1AA, 8, true, false, false, 0x01, false);

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

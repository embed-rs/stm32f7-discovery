#[derive(Debug)]
pub enum Error {
    Error,
    NoSdCard,
    Timeout,
    CardError { t: CardStatusFlags },
    SdmmcError { t: SdmmcErrorType },
}

#[derive(Debug)]
pub enum SdmmcErrorType {
    CmdCrcFailed,
    CmdRespTimeout,
}

bitflags! {
    pub flags CardStatusFlags: u32 {
        // Errors bits
        const OCR_ERROR_BITS        = 0xFDFFE008,
        const AKE_SEQ_ERROR         = 0x00000008,
        const ERASE_RESET           = 0x00002000,
        const CARD_ECC_DISABLED     = 0x00004000,
        const WP_ERASE_SKIP         = 0x00008000,
        const CID_CSD_OVERWRITE     = 0x00010000,
        const ERROR                 = 0x00080000,
        const CC_ERROR              = 0x00100000,
        const CARD_ECC_FAILED       = 0x00200000,
        const ILLEGAL_COMMAND       = 0x00400000,
        const COM_CRC_ERROR         = 0x00800000,
        const LOCK_UNLOCK_FAILED    = 0x01000000,
        const WP_VIOLATION          = 0x04000000,
        const ERASE_PARAM           = 0x08000000,
        const ERASE_SEQ_ERROR       = 0x10000000,
        const BLOCK_LEN_ERROR       = 0x20000000,
        const ADDRESS_MISALIGNED    = 0x40000000,
        const ADDRESS_OUT_OF_RANGE  = 0x80000000,

        // Other status bits
        const APP_CMD               = 0x00000020,
        const SWITCH_ERROR          = 0x00000080,
        const READY_FOR_DATA        = 0x00000100,
        const CURRENT_STATE         = 0x00001E00,
        const CARD_IS_LOCKED        = 0x02000000,
    }
}

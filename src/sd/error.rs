#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Error,                            // Unknown Error
    NoSdCard,                         // No SD Card
    Timeout,                          // Timeout while waiting for a response
    InvalidVoltrange,                 // Voltage Trial failed
    CardError { t: CardStatusFlags }, // Card Error, see CardStatusFlags
    SdmmcError { t: SdmmcErrorType }, // Response to a failed command
    RWError { t: RWErrorType },       // Error during reading from/writing to the card
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdmmcErrorType {
    CmdCrcFailed,   // CRC check failed
    CmdRespTimeout, // No response to command
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RWErrorType {
    AddressOutOfRange,
    DataTimeout,
    DataCrcFailed,
    TxUnderrun, // FIFO underrun
    RxOverrun,  // FIFO overrun
}

// See Documentation Table 207 and Table 228
bitflags! {
    pub flags CardStatusFlags: u32 {
        // Error bits
        const OCR_ERROR_BITS        = 0xFDFF_E008,
        const AKE_SEQ_ERROR         = 0x0000_0008,
        const ERASE_RESET           = 0x0000_2000,
        const CARD_ECC_DISABLED     = 0x0000_4000,
        const WP_ERASE_SKIP         = 0x0000_8000,
        const CID_CSD_OVERWRITE     = 0x0001_0000,
        const ERROR                 = 0x0008_0000,
        const CC_ERROR              = 0x0010_0000,
        const CARD_ECC_FAILED       = 0x0020_0000,
        const ILLEGAL_COMMAND       = 0x0040_0000,
        const COM_CRC_ERROR         = 0x0080_0000,
        const LOCK_UNLOCK_FAILED    = 0x0100_0000,
        const WP_VIOLATION          = 0x0400_0000,
        const ERASE_PARAM           = 0x0800_0000,
        const ERASE_SEQ_ERROR       = 0x1000_0000,
        const BLOCK_LEN_ERROR       = 0x2000_0000,
        const ADDRESS_MISALIGNED    = 0x4000_0000,
        const ADDRESS_OUT_OF_RANGE  = 0x8000_0000,

        // Other status bits
        const APP_CMD               = 0x0000_0020,
        const SWITCH_ERROR          = 0x0000_0080,
        const READY_FOR_DATA        = 0x0000_0100,
        const CURRENT_STATE         = 0x0000_1E00,
        const CARD_IS_LOCKED        = 0x0200_0000,

        // R6 errors
        const R6_GENERAL_UNKNOWN_ERROR  = 0x2000,
        const R6_ILLEGAL_COMMAND        = 0x4000,
        const R6_CRC_FAILED             = 0x8000,
    }
}

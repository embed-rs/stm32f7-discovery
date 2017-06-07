#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InterruptRequest {
    Wwdg = 0,           // Window Watchdog interrupt
    Pvd,                // PVD through EXTI line detection interrupt
    TampStamp,          // Tamper and TimeStamp interrupts through the EXTI line
    RtcWkup,            // RTC Wakeup interrupt through the EXTI line
    Flash,              // Flash global interrupt
    Rcc,                // RCC global interrupt
    Exti0,              // EXTI Line0 interrupt
    Exti1,              // EXTI Line1 interrupt
    Exti2,              // EXTI Line2 interrupt
    Exti3,              // EXTI Line3 interrupt
    Exti4,              // EXTI Line4 interrupt
    Dma1Stream0,        // DMA1 Stream0 global interrupt
    Dma1Stream1,        // DMA1 Stream1 global interrupt
    Dma1Stream2,        // DMA1 Stream2 global interrupt
    Dma1Stream3,        // DMA1 Stream3 global interrupt
    Dma1Stream4,        // DMA1 Stream4 global interrupt
    Dma1Stream5,        // DMA1 Stream5 global interrupt
    Dma1Stream6,        // DMA1 Stream6 global interrupt
    Adc,                // ADC1, ADC2 and ADC3 global interrupts
    Can1Tx,             // CAN1 TX interrupts
    Can1Rx0,            // CAN1 RX0 interrupts
    Can1Rx1,            // CAN1 RX1 interrupt
    Can1Sce,            // CAN1 SCE interrupt
    Exti5to9,           // EXTI Line[9:5] interrupts
    Tim1BrkTim9,        // TIM1 Break interrupt and TIM9 global interrupt
    Tim1UpTim10,        // TIM1 Update interrupt and TIM10 global interrupt
    Tim1TrgComTim11,    // TIM1 Trigger and Commutation interrupts and TIM11 global interrupt
    Tim1Cc,             // TIM1 Capture Compare interrupt
    Tim2,               // TIM2 global interrupt
    Tim3,               // TIM3 global interrupt
    Tim4,               // TIM4 global interrupt
    I2C1Ev,             // I2C1 event interrupt
    I2C1Er,             // I2C1 error interrupt
    I2C2Ev,             // I2C2 event interrupt
    I2C2Er,             // I2C2 error interrupt
    Spi1,               // SPI1 global interrupt
    Spi2,               // SPI2 global interrupt
    Usart1,             // USART1 global interrupt
    Usart2,             // USART2 global interrupt
    Usart3,             // USART3 global interrupt
    Exti10to15,         // EXTI Line[15:10] interrupts
    RtcAlarm,           // RTC Alarms (A and B) through EXTI line interrupt
    OtgFsWkup,          // USB On-The-Go FS Wakeup through EXTI line interrupt
    Tim8BrkTim12,       // TIM8 Break interrupt and TIM12 global interrupt
    Tim8UpTim13,        // TIM8 Update interrupt and TIM13 global interrupt
    Tim8TrgComTim14,    // TIM8 Trigger and Commutation interrupts and TIM14 global interrupt
    Tim8Cc,             // TIM8 Capture Compare interrupt
    DMA1Stream7,        // DMA1 Stream7 global interrupt
    Fsmc,               // FSMC global interrupt
    Sdmmc1,             // SDMMC1 global interrupt
    Tim5,               // TIM5 global interrupt
    Spi3,               // SPI3 global interrupt
    Uart4,              // UART4 global interrupt
    Uart5,              // UART5 global interrupt
    Tim6Dac,            // TIM6 global interrupt, DAC1 and DAC2 underrun error interrupts
    Tim7,               // TIM7 global interrupt
    DMA2Stream0,        // DMA2 Stream0 global interrupt
    DMA2Stream1,        // DMA2 Stream1 global interrupt
    DMA2Stream2,        // DMA2 Stream2 global interrupt
    DMA2Stream3,        // DMA2 Stream3 global interrupt
    DMA2Stream4,        // DMA2 Stream4 global interrupt
    Eth,                // Ethernet global interrupt
    EthWkup,            // Ethernet Wakeup through EXTI line interrupt
    Can2Tx,             // CAN2 TX interrupts
    Can2Rx0,            // CAN2 RX0 interrupts
    Can2Rx1,            // CAN2 RX1 interrupt
    Can2Sce,            // CAN2 SCE interrupt
    OtgFs,              // USB On The Go FS global interrupt
    DMA2Stream5,        // DMA2 Stream5 global interrupt
    DMA2Stream6,        // DMA2 Stream6 global interrupt
    DMA2Stream7,        // DMA2 Stream7 global interrupt
    Usart6,             // USART6 global interrupt
    I2C3Ev,             // I2C3 event interrupt
    I2C3Er,             // I2C3 error interrupt
    OtgHsEp1Out,        // USB On The Go HS End Point 1 Out global interrupt
    OtgHsEp1In,         // USB On The Go HS End Point 1 In global interrupt
    OtgHsWkup,          // USB On The Go HS Wakeup through EXTI interrupt
    OtgHs,              // USB On The Go HS global interrupt
    Dcmi,               // DCMI global interrupt
    Cryp,               // CRYP crypto global interrupt
    HashRng,            // Hash and Rng global interrupt
    Fpu,                // FPU global interrupt
    Uart7,              // UART7 global interrupt
    Uart8,              // UART8 global interrupt
    Spi4,               // SPI4 global interrupt
    Spi5,               // SPI5 global interrupt
    Spi6,               // SPI6 global interrupt
    Sai1,               // SAI1 global interrupt
    LcdTft,             // LCD-TFT global interrupt
    LcdTftError,        // LCD-TFT global Error interrupt
    Dma2D,              // DMA2D global interrupt
    Sai2,               // SAI2 global interrupt
    QuadSpi,            // QuadSPI global interrupt
    LpTimer1,           // LP Timer1 global interrupt
    HdmiCec,            // HDMI-CEC global interrupt
    I2C4Ev,             // I2C4 event interrupt
    I2C4Er,             // I2C4 Error interrupt
    Spdifrx,            // SPDIFRX global interrupt
}
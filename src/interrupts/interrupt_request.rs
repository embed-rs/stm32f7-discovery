#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InterruptRequest {
    /// Window Watchdog interrupt 
 	Wwdg = 0,
    /// PVD through EXTI line detection interrupt 
 	Pvd,
    /// Tamper and TimeStamp interrupts through the EXTI line 
 	TampStamp,
    /// RTC Wakeup interrupt through the EXTI line 
 	RtcWkup,
    /// Flash global interrupt 
 	Flash,
    /// RCC global interrupt 
 	Rcc,
    /// EXTI Line0 interrupt 
 	Exti0,
    /// EXTI Line1 interrupt 
 	Exti1,
    /// EXTI Line2 interrupt 
 	Exti2,
    /// EXTI Line3 interrupt 
 	Exti3,
    /// EXTI Line4 interrupt 
 	Exti4,
    /// DMA1 Stream0 global interrupt 
 	Dma1Stream0,
    /// DMA1 Stream1 global interrupt 
 	Dma1Stream1,
    /// DMA1 Stream2 global interrupt 
 	Dma1Stream2,
    /// DMA1 Stream3 global interrupt 
 	Dma1Stream3,
    /// DMA1 Stream4 global interrupt 
 	Dma1Stream4,
    /// DMA1 Stream5 global interrupt 
 	Dma1Stream5,
    /// DMA1 Stream6 global interrupt 
 	Dma1Stream6,
    /// ADC1, ADC2 and ADC3 global interrupts 
 	Adc,
    /// CAN1 TX interrupts 
 	Can1Tx,
    /// CAN1 RX0 interrupts 
 	Can1Rx0,
    /// CAN1 RX1 interrupt 
 	Can1Rx1,
    /// CAN1 SCE interrupt 
 	Can1Sce,
    /// EXTI Line[9:5] interrupts 
 	Exti5to9,
    /// TIM1 Break interrupt and TIM9 global interrupt 
 	Tim1BrkTim9,
    /// TIM1 Update interrupt and TIM10 global interrupt 
 	Tim1UpTim10,
    /// TIM1 Trigger and Commutation interrupts and TIM11 global interrupt 
 	Tim1TrgComTim11,
    /// TIM1 Capture Compare interrupt 
 	Tim1Cc,
    /// TIM2 global interrupt 
 	Tim2,
    /// TIM3 global interrupt 
 	Tim3,
    /// TIM4 global interrupt 
 	Tim4,
    /// I2C1 event interrupt 
 	I2C1Ev,
    /// I2C1 error interrupt 
 	I2C1Er,
    /// I2C2 event interrupt 
 	I2C2Ev,
    /// I2C2 error interrupt 
 	I2C2Er,
    /// SPI1 global interrupt 
 	Spi1,
    /// SPI2 global interrupt 
 	Spi2,
    /// USART1 global interrupt 
 	Usart1,
    /// USART2 global interrupt 
 	Usart2,
    /// USART3 global interrupt 
 	Usart3,
    /// EXTI Line[15:10] interrupts 
 	Exti10to15,
    /// RTC Alarms (A and B) through EXTI line interrupt 
 	RtcAlarm,
    /// USB On-The-Go FS Wakeup through EXTI line interrupt 
 	OtgFsWkup,
    /// TIM8 Break interrupt and TIM12 global interrupt 
 	Tim8BrkTim12,
    /// TIM8 Update interrupt and TIM13 global interrupt 
 	Tim8UpTim13,
    /// TIM8 Trigger and Commutation interrupts and TIM14 global interrupt 
 	Tim8TrgComTim14,
    /// TIM8 Capture Compare interrupt 
 	Tim8Cc,
    /// DMA1 Stream7 global interrupt 
 	DMA1Stream7,
    /// FSMC global interrupt 
 	Fsmc,
    /// SDMMC1 global interrupt 
 	Sdmmc1,
    /// TIM5 global interrupt 
 	Tim5,
    /// SPI3 global interrupt 
 	Spi3,
    /// UART4 global interrupt 
 	Uart4,
    /// UART5 global interrupt 
 	Uart5,
    /// TIM6 global interrupt, DAC1 and DAC2 underrun error interrupts 
 	Tim6Dac,
    /// TIM7 global interrupt 
 	Tim7,
    /// DMA2 Stream0 global interrupt 
 	DMA2Stream0,
    /// DMA2 Stream1 global interrupt 
 	DMA2Stream1,
    /// DMA2 Stream2 global interrupt 
 	DMA2Stream2,
    /// DMA2 Stream3 global interrupt 
 	DMA2Stream3,
    /// DMA2 Stream4 global interrupt 
 	DMA2Stream4,
    /// Ethernet global interrupt 
 	Eth,
    /// Ethernet Wakeup through EXTI line interrupt 
 	EthWkup,
    /// CAN2 TX interrupts 
 	Can2Tx,
    /// CAN2 RX0 interrupts 
 	Can2Rx0,
    /// CAN2 RX1 interrupt 
 	Can2Rx1,
    /// CAN2 SCE interrupt 
 	Can2Sce,
    /// USB On The Go FS global interrupt 
 	OtgFs,
    /// DMA2 Stream5 global interrupt 
 	DMA2Stream5,
    /// DMA2 Stream6 global interrupt 
 	DMA2Stream6,
    /// DMA2 Stream7 global interrupt 
 	DMA2Stream7,
    /// USART6 global interrupt 
 	Usart6,
    /// I2C3 event interrupt 
 	I2C3Ev,
    /// I2C3 error interrupt 
 	I2C3Er,
    /// USB On The Go HS End Point 1 Out global interrupt 
 	OtgHsEp1Out,
    /// USB On The Go HS End Point 1 In global interrupt 
 	OtgHsEp1In,
    /// USB On The Go HS Wakeup through EXTI interrupt 
 	OtgHsWkup,
    /// USB On The Go HS global interrupt 
 	OtgHs,
    /// DCMI global interrupt 
 	Dcmi,
    /// CRYP crypto global interrupt 
 	Cryp,
    /// Hash and Rng global interrupt 
 	HashRng,
    /// FPU global interrupt 
 	Fpu,
    /// UART7 global interrupt 
 	Uart7,
    /// UART8 global interrupt 
 	Uart8,
    /// SPI4 global interrupt 
 	Spi4,
    /// SPI5 global interrupt 
 	Spi5,
    /// SPI6 global interrupt 
 	Spi6,
    /// SAI1 global interrupt 
 	Sai1,
    /// LCD-TFT global interrupt 
 	LcdTft,
    /// LCD-TFT global Error interrupt 
 	LcdTftError,
    /// DMA2D global interrupt 
 	Dma2D,
    /// SAI2 global interrupt 
 	Sai2,
    /// QuadSPI global interrupt 
 	QuadSpi,
    /// LP Timer1 global interrupt 
 	LpTimer1,
    /// HDMI-CEC global interrupt 
 	HdmiCec,
    /// I2C4 event interrupt 
 	I2C4Ev,
    /// I2C4 Error interrupt 
 	I2C4Er,
    /// SPDIFRX global interrupt 
 	Spdifrx,
}

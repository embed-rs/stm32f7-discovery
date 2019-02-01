use self::pin_wrapper::PortPins;
use crate::gpio::{
    AlternateFunction, GpioPort, InputPin, OutputPin, OutputSpeed, OutputType, Resistor,
};
use stm32f7::stm32f7x6::{
    GPIOA, GPIOB, GPIOC, GPIOD, GPIOE, GPIOF, GPIOG, GPIOH, GPIOI, GPIOJ, GPIOK,
};

/// This struct contains special PIO pins.
pub struct Pins<
    Led: OutputPin,
    Button: InputPin,
    DisplayEnable: OutputPin,
    Backlight: OutputPin,
    SdcardPresent: InputPin,
    AudioIn: InputPin,
> {
    /// This pin enables or disables the debug LED.
    pub led: Led,
    /// This pin reports whether the user button is pressed.
    pub button: Button,
    /// This pin controls whether the LCD is enabled.
    pub display_enable: DisplayEnable,
    /// This pin controls the LCD backlight.
    pub backlight: Backlight,
    /// This pin reports whether there is a card in the SD card slot.
    pub sdcard_present: SdcardPresent,
    /// This pin reports whether there is new audio data from the microphone.
    ///
    /// **Does not work currently**
    pub audio_in: AudioIn,
}

/// Initializes the pin mapping for all the peripherals.
///
/// This function uses Rust's ownership mechanism internally to report duplicate mappings
/// at compile time.
pub fn init<'a>(
    mut gpio_a: GpioPort<GPIOA>,
    mut gpio_b: GpioPort<GPIOB>,
    mut gpio_c: GpioPort<GPIOC>,
    mut gpio_d: GpioPort<GPIOD>,
    mut gpio_e: GpioPort<GPIOE>,
    mut gpio_f: GpioPort<GPIOF>,
    mut gpio_g: GpioPort<GPIOG>,
    mut gpio_h: GpioPort<GPIOH>,
    mut gpio_i: GpioPort<GPIOI>,
    mut gpio_j: GpioPort<GPIOJ>,
    mut gpio_k: GpioPort<GPIOK>,
) -> Pins<
    impl OutputPin + 'a,
    impl InputPin + 'a,
    impl OutputPin + 'a,
    impl OutputPin + 'a,
    impl InputPin + 'a,
    impl InputPin + 'a,
> {
    let gpio_a_pins = PortPins::new();
    let gpio_b_pins = PortPins::new();
    let gpio_c_pins = PortPins::new();
    let gpio_d_pins = PortPins::new();
    let gpio_e_pins = PortPins::new();
    let gpio_f_pins = PortPins::new();
    let gpio_g_pins = PortPins::new();
    let gpio_h_pins = PortPins::new();
    let gpio_i_pins = PortPins::new();
    let gpio_j_pins = PortPins::new();
    let gpio_k_pins = PortPins::new();

    let led = gpio_i
        .to_output(
            gpio_i_pins.pin_1.pin(),
            OutputType::PushPull,
            OutputSpeed::Low,
            Resistor::NoPull,
        )
        .expect("Pin I-1 already in use");
    let button = gpio_i
        .to_input(gpio_i_pins.pin_11.pin(), Resistor::NoPull)
        .expect("Pin I-11 already in use");
    let _lcd_int = gpio_i
        .to_input(gpio_i_pins.pin_13.pin(), Resistor::NoPull)
        .expect("Pin I-13 already in use");

    // sdram pins
    {
        let alt_fn = AlternateFunction::AF12;
        let speed = OutputSpeed::High;
        let typ = OutputType::PushPull;
        let res = Resistor::PullUp;

        let b_pins = &[
            gpio_b_pins.pin_5.pin(), // sdcke1
        ];
        let c_pins = &[
            gpio_c_pins.pin_3.pin(), // sdcke0
        ];
        let d_pins = &[
            gpio_d_pins.pin_14.pin(), // d0
            gpio_d_pins.pin_15.pin(), // d1
            gpio_d_pins.pin_0.pin(),  // d2
            gpio_d_pins.pin_1.pin(),  // d3
            gpio_d_pins.pin_8.pin(),  // d13
            gpio_d_pins.pin_9.pin(),  // d14
            gpio_d_pins.pin_10.pin(), // d15
        ];
        let e_pins = &[
            gpio_e_pins.pin_7.pin(),  // d4
            gpio_e_pins.pin_8.pin(),  // d5
            gpio_e_pins.pin_9.pin(),  // d6
            gpio_e_pins.pin_10.pin(), // d7
            gpio_e_pins.pin_11.pin(), // d8
            gpio_e_pins.pin_12.pin(), // d9
            gpio_e_pins.pin_13.pin(), // d10
            gpio_e_pins.pin_14.pin(), // d11
            gpio_e_pins.pin_15.pin(), // d12
        ];
        let f_pins = &[
            gpio_f_pins.pin_0.pin(),  // a0
            gpio_f_pins.pin_1.pin(),  // a1
            gpio_f_pins.pin_2.pin(),  // a2
            gpio_f_pins.pin_3.pin(),  // a3
            gpio_f_pins.pin_4.pin(),  // a4
            gpio_f_pins.pin_5.pin(),  // a5
            gpio_f_pins.pin_12.pin(), // a6
            gpio_f_pins.pin_13.pin(), // a7
            gpio_f_pins.pin_14.pin(), // a8
            gpio_f_pins.pin_15.pin(), // a9
            gpio_f_pins.pin_11.pin(), // nras
        ];
        let g_pins = &[
            gpio_g_pins.pin_0.pin(),  // a10
            gpio_g_pins.pin_1.pin(),  // a11
            gpio_g_pins.pin_2.pin(),  // a12
            gpio_g_pins.pin_4.pin(),  // ba0
            gpio_g_pins.pin_5.pin(),  // ba1
            gpio_g_pins.pin_8.pin(),  // sdclk
            gpio_g_pins.pin_15.pin(), // ncas
        ];
        let h_pins = &[
            gpio_h_pins.pin_3.pin(), // sdne0
            gpio_h_pins.pin_6.pin(), // sdne1
            gpio_h_pins.pin_5.pin(), // sdnwe
        ];

        gpio_b
            .to_alternate_function_all(b_pins, alt_fn, typ, speed, res)
            .expect("failed to reserve SDRAM GPIO B pins");
        gpio_c
            .to_alternate_function_all(c_pins, alt_fn, typ, speed, res)
            .expect("failed to reserve SDRAM GPIO C pins");
        gpio_d
            .to_alternate_function_all(d_pins, alt_fn, typ, speed, res)
            .expect("failed to reserve SDRAM GPIO D pins");
        gpio_e
            .to_alternate_function_all(e_pins, alt_fn, typ, speed, res)
            .expect("failed to reserve SDRAM GPIO E pins");
        gpio_f
            .to_alternate_function_all(f_pins, alt_fn, typ, speed, res)
            .expect("failed to reserve SDRAM GPIO F pins");
        gpio_g
            .to_alternate_function_all(g_pins, alt_fn, typ, speed, res)
            .expect("failed to reserve SDRAM GPIO G pins");
        gpio_h
            .to_alternate_function_all(h_pins, alt_fn, typ, speed, res)
            .expect("failed to reserve SDRAM GPIO H pins");
    }

    // lcd pins
    let (display_enable, backlight) = {
        let alt_fn = AlternateFunction::AF14;
        let speed = OutputSpeed::High;
        let typ = OutputType::PushPull;
        let res = Resistor::NoPull;

        let e_pins = &[
            gpio_e_pins.pin_4.pin(), // b0
        ];
        let g_pins = &[
            gpio_g_pins.pin_12.pin(), // b4
        ];
        let i_pins = &[
            gpio_i_pins.pin_15.pin(), // r0
            gpio_i_pins.pin_14.pin(), // clk
            gpio_i_pins.pin_10.pin(), // hsync
            gpio_i_pins.pin_9.pin(),  // vsync
        ];
        let j_pins = &[
            gpio_j_pins.pin_0.pin(),  // r1
            gpio_j_pins.pin_1.pin(),  // r2
            gpio_j_pins.pin_2.pin(),  // r3
            gpio_j_pins.pin_3.pin(),  // r4
            gpio_j_pins.pin_4.pin(),  // r5
            gpio_j_pins.pin_5.pin(),  // r6
            gpio_j_pins.pin_6.pin(),  // r7
            gpio_j_pins.pin_7.pin(),  // g0
            gpio_j_pins.pin_8.pin(),  // g1
            gpio_j_pins.pin_9.pin(),  // g2
            gpio_j_pins.pin_10.pin(), // g3
            gpio_j_pins.pin_11.pin(), // g4
            gpio_j_pins.pin_13.pin(), // b1
            gpio_j_pins.pin_14.pin(), // b2
            gpio_j_pins.pin_15.pin(), // b3
        ];
        let k_pins = &[
            gpio_k_pins.pin_0.pin(), // g5
            gpio_k_pins.pin_1.pin(), // g6
            gpio_k_pins.pin_2.pin(), // g7
            gpio_k_pins.pin_4.pin(), // b5
            gpio_k_pins.pin_5.pin(), // b6
            gpio_k_pins.pin_6.pin(), // b7
            gpio_k_pins.pin_7.pin(), // data_enable
        ];

        gpio_e
            .to_alternate_function_all(e_pins, alt_fn, typ, speed, res)
            .expect("Failed to reserve LCD GPIO E pins");
        gpio_g
            .to_alternate_function_all(g_pins, alt_fn, typ, speed, res)
            .expect("Failed to reserve LCD GPIO G pins");
        gpio_i
            .to_alternate_function_all(i_pins, alt_fn, typ, speed, res)
            .expect("Failed to reserve LCD GPIO I pins");
        gpio_j
            .to_alternate_function_all(j_pins, alt_fn, typ, speed, res)
            .expect("Failed to reserve LCD GPIO J pins");
        gpio_k
            .to_alternate_function_all(k_pins, alt_fn, typ, speed, res)
            .expect("Failed to reserve LCD GPIO K pins");

        let display_enable = gpio_i
            .to_output(
                gpio_i_pins.pin_12.pin(),
                OutputType::PushPull,
                OutputSpeed::Low,
                Resistor::PullDown,
            )
            .expect("Failed to reserve LCD display enable pin");
        let backlight = gpio_k
            .to_output(
                gpio_k_pins.pin_3.pin(),
                OutputType::PushPull,
                OutputSpeed::Low,
                Resistor::PullDown,
            )
            .expect("Failed to reserve LCD backlight pin");
        (display_enable, backlight)
    };

    // i2c pins
    {
        let alt_fn = AlternateFunction::AF4;
        let speed = OutputSpeed::Medium;
        let typ = OutputType::OpenDrain;
        let res = Resistor::PullUp;

        let b_pins = &[
            gpio_b_pins.pin_6.pin(),  // i2c1_scl
            gpio_b_pins.pin_7.pin(),  // i2c1_sda
            gpio_b_pins.pin_10.pin(), // i2c2_scl
            gpio_b_pins.pin_11.pin(), // i2c2_sda
        ];
        let d_pins = &[
            gpio_d_pins.pin_13.pin(), // i2c4_sda
        ];
        let h_pins = &[
            gpio_h_pins.pin_7.pin(),  // i2c3_scl
            gpio_h_pins.pin_8.pin(),  // i2c3_sda
            gpio_h_pins.pin_11.pin(), // i2c4_scl
        ];

        gpio_b
            .to_alternate_function_all(b_pins, alt_fn, typ, speed, res)
            .expect("Failed to reserve I2C GPIO B pins");
        gpio_d
            .to_alternate_function_all(d_pins, alt_fn, typ, speed, res)
            .expect("Failed to reserve I2C GPIO D pins");
        gpio_h
            .to_alternate_function_all(h_pins, alt_fn, typ, speed, res)
            .expect("Failed to reserve I2C GPIO H pins");
    }

    // sai2 pins
    let audio_in = {
        let alt_fn = AlternateFunction::AF10;
        let speed = OutputSpeed::High;
        let typ = OutputType::PushPull;
        let res = Resistor::NoPull;

        // block A (master)
        let i_pins = &[
            gpio_i_pins.pin_7.pin(), // sai2_fs_a
            gpio_i_pins.pin_5.pin(), // sai2_sck_a
            gpio_i_pins.pin_6.pin(), // sai2_sd_a
            gpio_i_pins.pin_4.pin(), // sai2_mclk_a
        ];
        // block B (synchronous slave)
        let g_pins = &[
            gpio_g_pins.pin_10.pin(), // sai2_sd_b
        ];

        gpio_i
            .to_alternate_function_all(i_pins, alt_fn, typ, speed, res)
            .expect("Failed to reserve SAI2 GPIO I pins");
        gpio_g
            .to_alternate_function_all(g_pins, alt_fn, typ, speed, res)
            .expect("Failed to reserve SAI2 GPIO G pins");

        let audio_in = gpio_h
            .to_input(gpio_h_pins.pin_15.pin(), Resistor::NoPull)
            .expect("Failed to reserve SAI2 audio in pin");
        audio_in
    };

    // SD card pins
    let sdcard_present = {
        let alt_fn = AlternateFunction::AF12;
        let speed = OutputSpeed::High;
        let typ = OutputType::PushPull;
        let res = Resistor::PullUp;

        // dx = data ports. For Default Bus mode only d0 is needed.
        let b_pins = &[
            gpio_b_pins.pin_8.pin(), // d4
            gpio_b_pins.pin_9.pin(), // d5
        ];
        let c_pins = &[
            gpio_c_pins.pin_8.pin(),  // d0
            gpio_c_pins.pin_9.pin(),  // d1
            gpio_c_pins.pin_10.pin(), // d2
            gpio_c_pins.pin_11.pin(), // d3
            gpio_c_pins.pin_6.pin(),  // d6
            gpio_c_pins.pin_7.pin(),  // d7
            gpio_c_pins.pin_12.pin(), // ck (clock)
        ];
        let d_pins = &[
            gpio_d_pins.pin_2.pin(), // cmd
        ];

        gpio_b
            .to_alternate_function_all(b_pins, alt_fn, typ, speed, res)
            .expect("Failed to reserve SD card GPIO B pins");
        gpio_c
            .to_alternate_function_all(c_pins, alt_fn, typ, speed, res)
            .expect("Failed to reserve SD card GPIO C pins");
        gpio_d
            .to_alternate_function_all(d_pins, alt_fn, typ, speed, res)
            .expect("Failed to reserve SD card GPIO D pins");

        let present_pin = gpio_c
            .to_input(gpio_c_pins.pin_13.pin(), Resistor::PullUp)
            .expect("Failed to reserve SD card present pin");
        present_pin
    };

    // ethernet pins
    {
        let alt_fn = AlternateFunction::AF11;
        let speed = OutputSpeed::High;
        let typ = OutputType::PushPull;
        let res = Resistor::NoPull;

        // RMII pins
        let a_pins = &[
            gpio_a_pins.pin_1.pin(), // ref_clk
            gpio_a_pins.pin_2.pin(), // mdio
            gpio_a_pins.pin_7.pin(), // crsdv
        ];
        let c_pins = &[
            gpio_c_pins.pin_1.pin(), // mdc
            gpio_c_pins.pin_4.pin(), // rxd0
            gpio_c_pins.pin_5.pin(), // rxd1
        ];
        let g_pins = &[
            gpio_g_pins.pin_11.pin(), // tx_en
            gpio_g_pins.pin_13.pin(), // txd0
            gpio_g_pins.pin_14.pin(), // txd1
        ];

        gpio_a
            .to_alternate_function_all(a_pins, alt_fn, typ, speed, res)
            .expect("Failed to reserve ethernet GPIO A pins");
        gpio_c
            .to_alternate_function_all(c_pins, alt_fn, typ, speed, res)
            .expect("Failed to reserve ethernet GPIO C pins");
        gpio_g
            .to_alternate_function_all(g_pins, alt_fn, typ, speed, res)
            .expect("Failed to reserve ethernet GPIO G pins");
    }

    Pins {
        led,
        button,
        display_enable,
        backlight,
        sdcard_present,
        audio_in,
    }
}

/// Helper structs for catching double uses of pins at compile time.
mod pin_wrapper {
    use crate::gpio::PinNumber;

    #[allow(dead_code)]
    pub(super) struct PortPins {
        pub(super) pin_0: PinWrapper,
        pub(super) pin_1: PinWrapper,
        pub(super) pin_2: PinWrapper,
        pub(super) pin_3: PinWrapper,
        pub(super) pin_4: PinWrapper,
        pub(super) pin_5: PinWrapper,
        pub(super) pin_6: PinWrapper,
        pub(super) pin_7: PinWrapper,
        pub(super) pin_8: PinWrapper,
        pub(super) pin_9: PinWrapper,
        pub(super) pin_10: PinWrapper,
        pub(super) pin_11: PinWrapper,
        pub(super) pin_12: PinWrapper,
        pub(super) pin_13: PinWrapper,
        pub(super) pin_14: PinWrapper,
        pub(super) pin_15: PinWrapper,
    }

    impl PortPins {
        pub(super) fn new() -> PortPins {
            use crate::gpio::PinNumber::*;

            PortPins {
                pin_0: PinWrapper(Pin0),
                pin_1: PinWrapper(Pin1),
                pin_2: PinWrapper(Pin2),
                pin_3: PinWrapper(Pin3),
                pin_4: PinWrapper(Pin4),
                pin_5: PinWrapper(Pin5),
                pin_6: PinWrapper(Pin6),
                pin_7: PinWrapper(Pin7),
                pin_8: PinWrapper(Pin8),
                pin_9: PinWrapper(Pin9),
                pin_10: PinWrapper(Pin10),
                pin_11: PinWrapper(Pin11),
                pin_12: PinWrapper(Pin12),
                pin_13: PinWrapper(Pin13),
                pin_14: PinWrapper(Pin14),
                pin_15: PinWrapper(Pin15),
            }
        }
    }

    /// A non-copy wrapper for a PinNumber.
    pub(super) struct PinWrapper(PinNumber);

    impl PinWrapper {
        pub(super) fn pin(self) -> PinNumber {
            self.0
        }
    }
}

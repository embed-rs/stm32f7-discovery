use self::pin_wrapper::PortPins;
use gpio::{
    AlternateFunction, GpioPort, InputPin, OutputPin, OutputSpeed, OutputType, RegisterBlockA,
    RegisterBlockB, RegisterBlockD, Resistor,
};

pub struct Pins<LED: OutputPin, BUTTON: InputPin> {
    pub led: LED,
    pub button: BUTTON,
}

pub fn init<'a>(
    _gpio_a: GpioPort<RegisterBlockA<'a>>,
    mut gpio_b: GpioPort<RegisterBlockB<'a>>,
    mut gpio_c: GpioPort<RegisterBlockD<'a>>,
    mut gpio_d: GpioPort<RegisterBlockD<'a>>,
    mut gpio_e: GpioPort<RegisterBlockD<'a>>,
    mut gpio_f: GpioPort<RegisterBlockD<'a>>,
    mut gpio_g: GpioPort<RegisterBlockD<'a>>,
    mut gpio_h: GpioPort<RegisterBlockD<'a>>,
    mut gpio_i: GpioPort<RegisterBlockD<'a>>,
    _gpio_j: GpioPort<RegisterBlockD<'a>>,
    _gpio_k: GpioPort<RegisterBlockD<'a>>,
) -> Pins<impl OutputPin + 'a, impl InputPin + 'a> {
    let _gpio_a_pins = PortPins::new();
    let gpio_b_pins = PortPins::new();
    let gpio_c_pins = PortPins::new();
    let gpio_d_pins = PortPins::new();
    let gpio_e_pins = PortPins::new();
    let gpio_f_pins = PortPins::new();
    let gpio_g_pins = PortPins::new();
    let gpio_h_pins = PortPins::new();
    let gpio_i_pins = PortPins::new();
    let _gpio_j_pins = PortPins::new();
    let _gpio_k_pins = PortPins::new();

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

    Pins { led, button }
}

/// Helper structs for catching double uses of pins at compile time.
mod pin_wrapper {
    use gpio::PinNumber;

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
            use gpio::PinNumber::*;

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

use self::pin_wrapper::PortPins;
use gpio::{GpioPort, InputPin, OutputPin, OutputSpeed, OutputType, RegisterBlockD, Resistor};

pub struct Pins<LED: OutputPin, BUTTON: InputPin> {
    pub led: LED,
    pub button: BUTTON,
}

pub fn init<'a>(
    mut gpio_i: GpioPort<RegisterBlockD<'a>>,
) -> Pins<impl OutputPin + 'a, impl InputPin + 'a> {
    let gpio_i_pins = PortPins::new();

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

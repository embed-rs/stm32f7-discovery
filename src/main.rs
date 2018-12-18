#![feature(alloc)]
#![feature(alloc_error_handler)]
#![feature(generators, generator_trait)]
#![feature(pin, futures_api)]
#![feature(arbitrary_self_types)]
#![feature(async_await)]
#![no_main]
#![no_std]

#[macro_use]
extern crate alloc;
extern crate alloc_cortex_m;
extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate cortex_m_semihosting as sh;
#[macro_use]
extern crate stm32f7;
#[macro_use]
extern crate stm32f7_discovery;
extern crate smoltcp;
extern crate futures;
extern crate spin;

use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc_cortex_m::CortexMHeap;
use core::alloc::Layout as AllocLayout;
use core::fmt::Write;
use core::panic::PanicInfo;
use cortex_m::{asm, interrupt};
use rt::{entry, exception, ExceptionFrame};
use sh::hio::{self, HStdout};
use smoltcp::{
    socket::{
        Socket, SocketSet, TcpSocket, TcpSocketBuffer, UdpPacketMetadata, UdpSocket,
        UdpSocketBuffer,
    },
    time::Instant,
    wire::{EthernetAddress, IpAddress, IpEndpoint, Ipv4Address},
};
use stm32f7::stm32f7x6::{self, CorePeripherals, Interrupt, Peripherals, SAI2};
use stm32f7_discovery::{
    ethernet,
    gpio::{GpioPort, InputPin, OutputPin},
    init,
    lcd::{self, Color, Layer, Framebuffer, AudioWriter},
    random::Rng,
    sd,
    system_clock::{self, Hz},
    touch,
    task_runtime,
    interrupts::{self, InterruptRequest, Priority},
    future_mutex::FutureMutex,
    i2c::I2C,
};
use core::ops::{Generator, GeneratorState};
use core::future::Future;
use futures::{Stream, StreamExt};
use pin_utils::pin_mut;
use spin::Mutex;
use alloc::sync::Arc;
use alloc::collections::VecDeque;
use core::task::{Poll, LocalWaker};
use core::pin::Pin;

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

const HEAP_SIZE: usize = 50 * 1024; // in bytes
const ETH_ADDR: EthernetAddress = EthernetAddress([0x00, 0x08, 0xdc, 0xab, 0xcd, 0xef]);
const IP_ADDR: Ipv4Address = Ipv4Address([141, 52, 46, 198]);

#[entry]
fn main() -> ! {
    run()
}

fn run() -> ! {
    let core_peripherals = CorePeripherals::take().unwrap();
    let mut systick = core_peripherals.SYST;
    let mut nvic = core_peripherals.NVIC;

    let peripherals = Peripherals::take().unwrap();
    let mut rcc = peripherals.RCC;
    let mut pwr = peripherals.PWR;
    let mut flash = peripherals.FLASH;
    let mut fmc = peripherals.FMC;
    let mut ltdc = peripherals.LTDC;
    let mut sai_2 = peripherals.SAI2;
    let mut rng = peripherals.RNG;
    let mut sdmmc = peripherals.SDMMC1;
    let mut syscfg = peripherals.SYSCFG;
    let mut ethernet_mac = peripherals.ETHERNET_MAC;
    let mut ethernet_dma = peripherals.ETHERNET_DMA;
    let mut nvic_stir = peripherals.NVIC_STIR;
    let mut tim6 = peripherals.TIM6;
    let mut exti = peripherals.EXTI;

    init::init_system_clock_216mhz(&mut rcc, &mut pwr, &mut flash);
    init::enable_gpio_ports(&mut rcc);
    init::enable_syscfg(&mut rcc);

    let gpio_a = GpioPort::new_a(&peripherals.GPIOA);
    let gpio_b = GpioPort::new_b(&peripherals.GPIOB);
    let gpio_c = GpioPort::new(&peripherals.GPIOC);
    let gpio_d = GpioPort::new(&peripherals.GPIOD);
    let gpio_e = GpioPort::new(&peripherals.GPIOE);
    let gpio_f = GpioPort::new(&peripherals.GPIOF);
    let gpio_g = GpioPort::new(&peripherals.GPIOG);
    let gpio_h = GpioPort::new(&peripherals.GPIOH);
    let gpio_i = GpioPort::new(&peripherals.GPIOI);
    let gpio_j = GpioPort::new(&peripherals.GPIOJ);
    let gpio_k = GpioPort::new(&peripherals.GPIOK);
    let mut pins = init::pins(
        gpio_a, gpio_b, gpio_c, gpio_d, gpio_e, gpio_f, gpio_g, gpio_h, gpio_i, gpio_j, gpio_k,
    );

    // configures the system timer to trigger a SysTick exception every second
    init::init_systick(Hz(100), &mut systick, &rcc);
    systick.enable_interrupt();

    init::init_sdram(&mut rcc, &mut fmc);
    let mut lcd = init::init_lcd(&mut ltdc, &mut rcc);
    pins.display_enable.set(true);
    pins.backlight.set(true);

    // Initialize the allocator BEFORE you use it
    unsafe { ALLOCATOR.init(rt::heap_start() as usize, HEAP_SIZE) }

    lcd.set_background_color(Color::from_hex(0x006600));
    let mut layer_1 = lcd.layer_1().unwrap();
    let mut layer_2 = lcd.layer_2().unwrap();

    layer_2.clear();
    lcd::init_stdout(layer_2);

    println!("Hello World");


    let xs = vec![1, 2, 3];

    let mut i2c_3 = init::init_i2c_3(Box::leak(Box::new(peripherals.I2C3)), &mut rcc);
    i2c_3.test_1();
    i2c_3.test_2();

    nvic.enable(Interrupt::EXTI0);

    let mut sd = sd::Sd::new(&mut sdmmc, &mut rcc, &pins.sdcard_present);

    init::init_sai_2(&mut sai_2, &mut rcc);
    init::init_wm8994(&mut i2c_3).expect("WM8994 init failed");
    // touch initialization should be done after audio initialization, because the touch
    // controller might not be ready yet
    touch::check_family_id(&mut i2c_3).unwrap();

    let mut rng = Rng::init(&mut rng, &mut rcc).expect("RNG init failed");
    print!("Random numbers: ");
    for _ in 0..4 {
        print!(
            "{} ",
            rng.poll_and_get()
                .expect("Failed to generate random number")
        );
    }
    println!("");

    // enable timers
    rcc.apb1enr.modify(|_, w| w.tim6en().enabled());

    // configure timer
    // clear update event
    tim6.sr.modify(|_, w| w.uif().clear_bit());

    // setup timing
    tim6.psc.modify(|_, w| unsafe { w.psc().bits(42000) });
    tim6.arr.modify(|_, w| unsafe { w.arr().bits(3000) });

    // enable interrupt
    tim6.dier.modify(|_, w| w.uie().set_bit());
    // start the timer counter
    tim6.cr1.modify(|_, w| w.cen().set_bit());

    interrupts::scope(
        &mut nvic,
        &mut nvic_stir,
        |_| {},
        |interrupt_table| {
            use futures::{
                channel::mpsc,
                task::LocalSpawnExt,
                StreamExt,
            };

            let (idle_waker_sink, mut idle_waker_stream) = mpsc::unbounded();
            let (tim6_sink, mut tim6_stream) = mpsc::unbounded();
            let (button_sink, mut button_stream) = mpsc::unbounded();
            let (touch_int_sink, mut touch_int_stream) = mpsc::unbounded();

            interrupt_table.register(InterruptRequest::TIM6_DAC, Priority::P1, move || {
                tim6_sink.unbounded_send(()).expect("sending on tim6 channel failed");
                let tim = &mut tim6;
                // make sure the interrupt doesn't just restart again by clearing the flag
                tim.sr.modify(|_, w| w.uif().clear_bit());
            }).expect("registering tim6 interrupt failed");

            // choose pin I-11 for exti11 line
            syscfg.exticr3.modify(|_, w| unsafe { w.exti11().bits(0b1000) });
            // trigger exti11 on rising
            exti.rtsr.modify(|_, w| w.tr11().set_bit());
            // unmask exti11 line
            exti.imr.modify(|_, w| w.mr11().set_bit());

            // choose pin I-13 for exti13 line
            syscfg.exticr4.modify(|_, w| unsafe { w.exti13().bits(0b1000) });
            // trigger exti13 on rising
            exti.rtsr.modify(|_, w| w.tr13().set_bit());
            // unmask exti13 line
            exti.imr.modify(|_, w| w.mr13().set_bit());

            // choose pin H-15 for exti15 line
            syscfg.exticr4.modify(|_, w| unsafe { w.exti15().bits(0b0111) });
            // trigger exti15 on rising
            exti.rtsr.modify(|_, w| w.tr15().set_bit());
            // unmask exti15 line
            exti.imr.modify(|_, w| w.mr15().set_bit());


            interrupt_table.register(InterruptRequest::EXTI15_10, Priority::P1, move || {
                exti.pr.modify(|r, w| {
                    if r.pr11().bit_is_set() {
                        button_sink.unbounded_send(()).expect("sending on button channel failed");
                        w.pr11().set_bit();
                    } else if r.pr13().bit_is_set() {
                        touch_int_sink.unbounded_send(()).expect("sending on touch_int channel failed");
                        w.pr13().set_bit();
                    } else {
                        panic!("unknown exti15_10 interrupt");
                    }
                    w
                });
            }).expect("registering exti15_10 interrupt failed");



            interrupt_table.register(InterruptRequest::EXTI9_5, Priority::P1, move || {
                panic!("unknown exti9_5 interrupt");
            }).expect("registering exti9_5 interrupt failed");

            // tasks

            let idle_stream = task_runtime::IdleStream::new(idle_waker_sink.clone());

            // ethernet
            let mut ethernet_task_idle_stream = task_runtime::IdleStream::new(idle_waker_sink.clone());
            let ethernet_task = async move || {
                let mut ethernet_interface = ethernet::EthernetDevice::new(
                    Default::default(),
                    Default::default(),
                    &mut rcc,
                    &mut syscfg,
                    &mut ethernet_mac,
                    &mut ethernet_dma,
                    ETH_ADDR,
                )
                .map(|device| device.into_interface(IP_ADDR));
                if let Err(e) = ethernet_interface {
                    println!("ethernet init failed: {:?}", e);
                };

                let mut sockets = SocketSet::new(Vec::new());

                if ethernet_interface.is_ok() {
                    let endpoint = IpEndpoint::new(IpAddress::Ipv4(IP_ADDR), 15);
                    let udp_rx_buffer = UdpSocketBuffer::new(vec![UdpPacketMetadata::EMPTY; 3], vec![0u8; 256]);
                    let udp_tx_buffer = UdpSocketBuffer::new(vec![UdpPacketMetadata::EMPTY; 1], vec![0u8; 128]);
                    let mut example_udp_socket = UdpSocket::new(udp_rx_buffer, udp_tx_buffer);
                    example_udp_socket.bind(endpoint).unwrap();
                    sockets.add(example_udp_socket);

                    let tcp_rx_buffer = TcpSocketBuffer::new(vec![0; ethernet::MTU]);
                    let tcp_tx_buffer = TcpSocketBuffer::new(vec![0; ethernet::MTU]);
                    let mut example_tcp_socket = TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer);
                    example_tcp_socket.listen(endpoint).unwrap();
                    sockets.add(example_tcp_socket);
                }

                // handle new ethernet packets
                if let Ok(ref mut eth) = ethernet_interface {
                    loop {
                        match eth.poll(
                            &mut sockets,
                            Instant::from_millis(system_clock::ms() as i64),
                        ) {
                            Err(::smoltcp::Error::Exhausted) => {
                                await!(ethernet_task_idle_stream.next()).expect("idle stream closed");
                            },
                            Err(::smoltcp::Error::Unrecognized) => {}
                            Err(e) => println!("Network error: {:?}", e),
                            Ok(socket_changed) => {
                                if socket_changed {
                                    for mut socket in sockets.iter_mut() {
                                        poll_socket(&mut socket).expect("socket poll failed");
                                    }
                                }
                            }
                        }
                    }
                }
            };

            let i2c_3_mutex = Arc::new(FutureMutex::new(i2c_3));
            let layer_1_mutex = Arc::new(FutureMutex::new(layer_1));

            let touch_task = TouchTask {
                touch_int_stream,
                i2c_3_mutex: i2c_3_mutex.clone(),
                layer_mutex: layer_1_mutex.clone(),
            };

            let audio_task = AudioTask::new(layer_1_mutex.clone(), sai_2, idle_stream.clone());

            let mut executor = task_runtime::Executor::new();
            executor.spawn_local(button_task(button_stream)).unwrap();
            executor.spawn_local(tim6_task(tim6_stream)).unwrap();
            executor.spawn_local(touch_task.run()).unwrap();
            executor.spawn_local(count_up_on_idle_task(idle_stream.clone())).unwrap();
            executor.spawn_local(audio_task.run()).unwrap();
            //executor.spawn_local(ethernet_task).unwrap();
            //executor.spawn_local(print_x);

            let mut idle = async move {
                loop {
                    let next_waker = await!(idle_waker_stream.next()).expect("idle channel closed");
                    next_waker.wake();
                }
            };

            executor.set_idle_task(idle);

            loop {
                executor.run();
                if pins.audio_in.get() == false {
                    println!("audio pin false");
                }
            }
        },
    );


    //let mut previous_button_state = pins.button.get();
    loop {
        /*
        // poll button state
        let current_button_state = pins.button.get();
        if current_button_state != previous_button_state {
            if current_button_state {
                pins.led.toggle();

                // trigger the `EXTI0` interrupt
                nvic.set_pending(Interrupt::EXTI0);
            }

            previous_button_state = current_button_state;
        }
        */

        //unsafe { audio_writer_task.resume() };


        // Initialize the SD Card on insert and deinitialize on extract.
        if sd.card_present() && !sd.card_initialized() {
            if let Some(i_err) = sd::init(&mut sd).err() {
                println!("{:?}", i_err);
            }
        } else if !sd.card_present() && sd.card_initialized() {
            sd::de_init(&mut sd);
        }
    }
}


async fn button_task(button_stream: impl Stream<Item=()>) {
    pin_mut!(button_stream);
    for i in 1usize.. {
        let next = await!(button_stream.next());
        assert!(next.is_some(), "button channel closed");
        print!("{}", i);
    }
}

async fn tim6_task(tim6_stream: impl Stream<Item=()>) {
    pin_mut!(tim6_stream);
    loop {
        let next = await!(tim6_stream.next());
        assert!(next.is_some(), "tim6 channel closed");
        print!("y");
    }
}

struct TouchTask<S, F>
    where S: Stream<Item=()>, F: Framebuffer,
{
    touch_int_stream: S,
    i2c_3_mutex: Arc<FutureMutex<I2C<'static>>>,
    layer_mutex: Arc<FutureMutex<Layer<F>>>,
}

impl<S, F> TouchTask<S, F> where S: Stream<Item=()>, F: Framebuffer, {
    async fn run(self) {
        let Self {touch_int_stream, i2c_3_mutex, layer_mutex} = self;
        pin_mut!(touch_int_stream);
        await!(layer_mutex.with(|l| l.clear()));
        loop {
            await!(touch_int_stream.next()).expect("touch channel closed");
            let touches = await!(i2c_3_mutex.with(|i2c_3| touch::touches(i2c_3))).unwrap();
            await!(layer_mutex.with(|layer| {
                for touch in touches {
                    layer.print_point_color_at(
                        touch.x as usize,
                        touch.y as usize,
                        Color::from_hex(0xffff00),
                    );
                }
            }))
        }
    }
}

struct AudioTask<F, S> where F: Framebuffer, S: Stream<Item=()> {
    sai_2: SAI2,
    idle_stream: S,
    audio_writer: AudioWriter<F>,
}

impl<F, S> AudioTask<F, S> where F: Framebuffer, S: Stream<Item=()> {
    fn new(layer_mutex: Arc<FutureMutex<Layer<F>>>, sai_2: SAI2, idle_stream: S) -> Self {
        Self {
            sai_2,
            idle_stream,
            audio_writer: AudioWriter::new(layer_mutex),
        }
    }

    async fn run(mut self) {
        let idle_stream = self.idle_stream;
        pin_mut!(idle_stream);

        let mut data0_buffer = None;
        loop {
            // FIXME: replace with actual interrupt stream when we get audio interrupts working
            await!(idle_stream.next());

            // poll for new audio data
            if self.sai_2.bsr.read().freq().bit_is_set() {
                // fifo_request_flag is set -> new data available
                let data = self.sai_2.bdr.read().data().bits();
                match data0_buffer {
                    None => {
                        data0_buffer = Some(data);
                    },
                    Some(data0) => {
                        let data1 = data;
                        await!(self.audio_writer.set_next_col(data0, data1));
                        data0_buffer = None;
                    }
                }
            }
        }
    }
}

async fn count_up_on_idle_task(idle_stream: impl Stream<Item=()>) {
    pin_mut!(idle_stream);
    let mut number = 0;
    loop {
        await!(idle_stream.next()).expect("idle stream closed");
        number += 1;
        if number % 100000 == 0 {
            print!(" idle({}) ", number);
        }
    }
}

fn poll_socket(socket: &mut Socket) -> Result<(), smoltcp::Error> {
    match socket {
        &mut Socket::Udp(ref mut socket) => match socket.endpoint().port {
            15 => loop {
                let reply;
                match socket.recv() {
                    Ok((data, remote_endpoint)) => {
                        let mut data = Vec::from(data);
                        let len = data.len() - 1;
                        data[..len].reverse();
                        reply = (data, remote_endpoint);
                    }
                    Err(smoltcp::Error::Exhausted) => break,
                    Err(err) => return Err(err),
                }
                socket.send_slice(&reply.0, reply.1)?;
            },
            _ => {}
        },
        &mut Socket::Tcp(ref mut socket) => match socket.local_endpoint().port {
            15 => {
                if !socket.may_recv() {
                    return Ok(());
                }
                let reply = socket.recv(|data| {
                    if data.len() > 0 {
                        let mut reply = Vec::from("tcp: ");
                        let start_index = reply.len();
                        reply.extend_from_slice(data);
                        reply[start_index..(start_index + data.len() - 1)].reverse();
                        (data.len(), Some(reply))
                    } else {
                        (data.len(), None)
                    }
                })?;
                if let Some(reply) = reply {
                    assert_eq!(socket.send_slice(&reply)?, reply.len());
                }
            }
            _ => {}
        },
        _ => {}
    }
    Ok(())
}

interrupt!(EXTI0, exti0, state: Option<HStdout> = None);

fn exti0(_state: &mut Option<HStdout>) {
    println!("Interrupt fired! This means that the button was pressed.");
}

#[exception]
fn SysTick() {
    system_clock::tick();
    // print a `.` every 500ms
    if system_clock::ticks() % 50 == 0 && lcd::stdout::is_initialized() {
        print!(".");
    }
}

#[exception]
fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("HardFault at {:#?}", ef);
}

// define what happens in an Out Of Memory (OOM) condition
#[alloc_error_handler]
fn rust_oom(_: AllocLayout) -> ! {
    panic!("out of memory");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    interrupt::disable();

    if lcd::stdout::is_initialized() {
        println!("{}", info);
    }

    if let Ok(mut hstdout) = hio::hstdout() {
        let _ = writeln!(hstdout, "{}", info);
    }

    // OK to fire a breakpoint here because we know the microcontroller is connected to a debugger
    asm::bkpt();

    loop {}
}

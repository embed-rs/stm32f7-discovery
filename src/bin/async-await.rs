#![feature(alloc)]
#![feature(alloc_error_handler)]
#![feature(generators, generator_trait)]
#![feature(futures_api)]
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
extern crate futures;
extern crate smoltcp;
extern crate spin;

use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc_cortex_m::CortexMHeap;
use core::alloc::Layout as AllocLayout;
use core::fmt::Write;
use core::panic::PanicInfo;
use cortex_m::{asm, interrupt};
use futures::{Stream, StreamExt};
use pin_utils::pin_mut;
use rt::{entry, exception, ExceptionFrame};
use sh::hio::{self, HStdout};
use smoltcp::{
    socket::{Socket, TcpSocket, TcpSocketBuffer, UdpPacketMetadata, UdpSocket, UdpSocketBuffer},
    time::Instant,
    wire::{EthernetAddress, IpEndpoint},
};
use stm32f7::stm32f7x6::{
    self as device, CorePeripherals, Interrupt, Peripherals, ETHERNET_DMA, ETHERNET_MAC, RCC, SAI2,
    SYSCFG,
};
use stm32f7_discovery::{
    ethernet,
    future_mutex::FutureMutex,
    gpio::{GpioPort, InputPin, OutputPin},
    i2c::I2C,
    init,
    interrupts::{self, InterruptRequest, Priority},
    lcd::{self, AudioWriter, Color, Framebuffer, Layer},
    random::Rng,
    sd,
    system_clock::{self, Hz},
    task_runtime, touch,
};

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

const HEAP_SIZE: usize = 50 * 1024; // in bytes
const ETH_ADDR: EthernetAddress = EthernetAddress([0x00, 0x08, 0xdc, 0xab, 0xcd, 0xef]);

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
    let syscfg = peripherals.SYSCFG;
    let ethernet_mac = peripherals.ETHERNET_MAC;
    let ethernet_dma = peripherals.ETHERNET_DMA;
    let mut nvic_stir = peripherals.NVIC_STIR;
    let mut tim6 = peripherals.TIM6;
    let exti = peripherals.EXTI;

    init::init_system_clock_216mhz(&mut rcc, &mut pwr, &mut flash);
    init::enable_gpio_ports(&mut rcc);
    init::enable_syscfg(&mut rcc);

    let gpio_a = GpioPort::new(peripherals.GPIOA);
    let gpio_b = GpioPort::new(peripherals.GPIOB);
    let gpio_c = GpioPort::new(peripherals.GPIOC);
    let gpio_d = GpioPort::new(peripherals.GPIOD);
    let gpio_e = GpioPort::new(peripherals.GPIOE);
    let gpio_f = GpioPort::new(peripherals.GPIOF);
    let gpio_g = GpioPort::new(peripherals.GPIOG);
    let gpio_h = GpioPort::new(peripherals.GPIOH);
    let gpio_i = GpioPort::new(peripherals.GPIOI);
    let gpio_j = GpioPort::new(peripherals.GPIOJ);
    let gpio_k = GpioPort::new(peripherals.GPIOK);
    let mut pins = init::pins(
        gpio_a, gpio_b, gpio_c, gpio_d, gpio_e, gpio_f, gpio_g, gpio_h, gpio_i, gpio_j, gpio_k,
    );

    // configures the system timer to trigger a SysTick exception every 10ms
    init::init_systick(Hz(100), &mut systick, &rcc);
    systick.enable_interrupt();

    init::init_sdram(&mut rcc, &mut fmc);
    let mut lcd = init::init_lcd(&mut ltdc, &mut rcc);
    pins.display_enable.set(true);
    pins.backlight.set(true);

    // Initialize the allocator BEFORE you use it
    unsafe { ALLOCATOR.init(rt::heap_start() as usize, HEAP_SIZE) }

    lcd.set_background_color(Color::from_hex(0x006600));
    let layer_1 = lcd.layer_1().unwrap();
    let mut layer_2 = lcd.layer_2().unwrap();

    layer_2.clear();

    // Make `println` print to the LCD
    lcd::init_stdout(layer_2);

    println!("Hello World");

    // example allocation
    let _xs = vec![1, 2, 3];

    let mut i2c_3 = init::init_i2c_3(peripherals.I2C3, &mut rcc);
    i2c_3.test_1();
    i2c_3.test_2();

    // TODO: is this needed?
    nvic.enable(Interrupt::EXTI0);

    // TODO: do something with this type
    let _sd = sd::Sd::new(&mut sdmmc, &mut rcc, &pins.sdcard_present);

    // audio initialization
    init::init_sai_2(&mut sai_2, &mut rcc);
    init::init_wm8994(&mut i2c_3).expect("WM8994 init failed");

    // touch initialization should be done after audio initialization, because the touch
    // controller might not be ready yet
    touch::check_family_id(&mut i2c_3).unwrap();

    // initialization of random number generator
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
            use futures::{task::LocalSpawnExt, StreamExt};
            use stm32f7_discovery::task_runtime::mpsc;

            // Future channels for passing interrupts events. The interrupt handler pushes
            // to a channel and the interrupt handler awaits the next item of the channel. There
            // is no data exchange, the item is always a zero sized `()`.
            // TODO: Currently we use futures::channel::mpsc, which means that we allocate heap
            // memory even though the item type is zero-sized. To avoid this we could build our
            // own channel type that uses an atomic counter instead of storing any items.
            let (idle_waker_sink, mut idle_waker_stream) = mpsc::unbounded();
            let (tim6_sink, tim6_stream) = mpsc::unbounded();
            let (button_sink, button_stream) = mpsc::unbounded();
            let (touch_int_sink, touch_int_stream) = mpsc::unbounded();

            // Interrupt handler for the TIM6_DAC interrupt, which is the interrupt triggered by
            // the tim6 timer.
            interrupt_table
                .register(InterruptRequest::TIM6_DAC, Priority::P1, move || {
                    tim6_sink
                        .unbounded_send(())
                        .expect("sending on tim6 channel failed");
                    let tim = &mut tim6;
                    // make sure the interrupt doesn't just restart again by clearing the flag
                    tim.sr.modify(|_, w| w.uif().clear_bit());
                })
                .expect("registering tim6 interrupt failed");

            // choose pin I-11 for exti11 line, which is the GPIO pin for the hardware button
            syscfg
                .exticr3
                .modify(|_, w| unsafe { w.exti11().bits(0b1000) });
            // trigger exti11 on rising
            exti.rtsr.modify(|_, w| w.tr11().set_bit());
            // unmask exti11 line
            exti.imr.modify(|_, w| w.mr11().set_bit());

            // choose pin I-13 for exti13 line, which is the GPIO pin signalizing a touch event
            syscfg
                .exticr4
                .modify(|_, w| unsafe { w.exti13().bits(0b1000) });
            // trigger exti13 on rising
            exti.rtsr.modify(|_, w| w.tr13().set_bit());
            // unmask exti13 line
            exti.imr.modify(|_, w| w.mr13().set_bit());

            // choose pin H-15 for exti15 line, which is the GPIO pin signalizing new audio data
            // TODO: the audio interrupt doesn't work yet
            syscfg
                .exticr4
                .modify(|_, w| unsafe { w.exti15().bits(0b0111) });
            // trigger exti15 on rising
            exti.rtsr.modify(|_, w| w.tr15().set_bit());
            // unmask exti15 line
            exti.imr.modify(|_, w| w.mr15().set_bit());

            // Interrupt handler for the EXTI15_10 interrupt, which is triggered by different
            // sources.
            interrupt_table
                .register(InterruptRequest::EXTI15_10, Priority::P1, move || {
                    exti.pr.modify(|r, w| {
                        if r.pr11().bit_is_set() {
                            button_sink
                                .unbounded_send(())
                                .expect("sending on button channel failed");
                            w.pr11().set_bit();
                        } else if r.pr13().bit_is_set() {
                            touch_int_sink
                                .unbounded_send(())
                                .expect("sending on touch_int channel failed");
                            w.pr13().set_bit();
                        } else {
                            panic!("unknown exti15_10 interrupt");
                        }
                        w
                    });
                })
                .expect("registering exti15_10 interrupt failed");

            let idle_stream = task_runtime::IdleStream::new(idle_waker_sink.clone());

            // ethernet
            let ethernet_task =
                EthernetTask::new(idle_stream.clone(), rcc, syscfg, ethernet_mac, ethernet_dma);

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
            executor
                .spawn_local(count_up_on_idle_task(idle_stream.clone()))
                .unwrap();
            executor.spawn_local(audio_task.run()).unwrap();

            //executor.spawn_local(print_x);

            // FIXME: Causes link error: no memory region specified for section '.ARM.extab'
            // see https://github.com/rust-embedded/cortex-m-rt/issues/157
            executor.spawn_local(ethernet_task.run()).unwrap();

            // FIXME: Does not work currently due to borrowing errors
            // executor.spawn_local(sd_card_task(sd, idle_stream.clone())).unwrap();

            let idle = async move {
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
    )
}

async fn button_task(button_stream: impl Stream<Item = ()>) {
    pin_mut!(button_stream);
    for i in 1usize.. {
        let next = await!(button_stream.next());
        assert!(next.is_some(), "button channel closed");
        print!("{}", i);
    }
}

async fn tim6_task(tim6_stream: impl Stream<Item = ()>) {
    pin_mut!(tim6_stream);
    loop {
        let next = await!(tim6_stream.next());
        assert!(next.is_some(), "tim6 channel closed");
        print!("y");
    }
}

struct TouchTask<S, F>
where
    S: Stream<Item = ()>,
    F: Framebuffer,
{
    touch_int_stream: S,
    i2c_3_mutex: Arc<FutureMutex<I2C<device::I2C3>>>,
    layer_mutex: Arc<FutureMutex<Layer<F>>>,
}

impl<S, F> TouchTask<S, F>
where
    S: Stream<Item = ()>,
    F: Framebuffer,
{
    async fn run(self) {
        let Self {
            touch_int_stream,
            i2c_3_mutex,
            layer_mutex,
        } = self;
        pin_mut!(touch_int_stream);
        await!(layer_mutex.with(|l| l.clear()));
        loop {
            await!(touch_int_stream.next()).expect("touch channel closed");
            let touches = await!(i2c_3_mutex.with(|i2c_3| touch::touches(i2c_3))).unwrap();
            await!(layer_mutex.with(|layer| for touch in touches {
                layer.print_point_color_at(
                    touch.x as usize,
                    touch.y as usize,
                    Color::from_hex(0xffff00),
                );
            }))
        }
    }
}

struct AudioTask<F, S>
where
    F: Framebuffer,
    S: Stream<Item = ()>,
{
    sai_2: SAI2,
    idle_stream: S,
    layer_mutex: Arc<FutureMutex<Layer<F>>>,
    audio_writer: AudioWriter,
}

impl<F, S> AudioTask<F, S>
where
    F: Framebuffer,
    S: Stream<Item = ()>,
{
    fn new(layer_mutex: Arc<FutureMutex<Layer<F>>>, sai_2: SAI2, idle_stream: S) -> Self {
        Self {
            sai_2,
            idle_stream,
            layer_mutex,
            audio_writer: AudioWriter::new(),
        }
    }

    async fn run(self) {
        let Self {
            idle_stream,
            layer_mutex,
            mut audio_writer,
            sai_2,
        } = self;
        pin_mut!(idle_stream);

        let mut data0_buffer = None;
        loop {
            // FIXME: replace with actual interrupt stream when we get audio interrupts working
            await!(idle_stream.next());

            // poll for new audio data
            if sai_2.bsr.read().freq().bit_is_set() {
                // fifo_request_flag is set -> new data available
                let data = sai_2.bdr.read().data().bits();
                match data0_buffer {
                    None => {
                        data0_buffer = Some(data);
                    }
                    Some(data0) => {
                        let data1 = data;
                        await!(layer_mutex.with(|l| audio_writer.set_next_col(l, data0, data1)));
                        data0_buffer = None;
                    }
                }
            }
        }
    }
}

async fn count_up_on_idle_task(idle_stream: impl Stream<Item = ()>) {
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

async fn sd_card_task<S, P>(mut sd: sd::Sd<'static, P>, idle_stream: S)
where
    S: Stream<Item = ()>,
    P: InputPin,
{
    pin_mut!(idle_stream);
    // Initialize the SD Card on insert and deinitialize on extract.
    loop {
        await!(idle_stream.next());
        if sd.card_present() && !sd.card_initialized() {
            if let Some(i_err) = sd::init(&mut sd).err() {
                println!("{:?}", i_err);
            }
        } else if !sd.card_present() && sd.card_initialized() {
            sd::de_init(&mut sd);
        }
    }
}

struct EthernetTask<S>
where
    S: Stream<Item = ()>,
{
    idle_stream: S,
    rcc: RCC,
    syscfg: SYSCFG,
    ethernet_mac: ETHERNET_MAC,
    ethernet_dma: ETHERNET_DMA,
}

impl<S> EthernetTask<S>
where
    S: Stream<Item = ()>,
{
    fn new(
        idle_stream: S,
        rcc: RCC,
        syscfg: SYSCFG,
        ethernet_mac: ETHERNET_MAC,
        ethernet_dma: ETHERNET_DMA,
    ) -> Self {
        Self {
            idle_stream,
            rcc,
            syscfg,
            ethernet_mac,
            ethernet_dma,
        }
    }

    async fn run(mut self) {
        use smoltcp::dhcp::Dhcpv4Client;
        use smoltcp::socket::SocketSet;
        use smoltcp::wire::{IpCidr, Ipv4Address};

        let ethernet_interface = ethernet::EthernetDevice::new(
            Default::default(),
            Default::default(),
            &mut self.rcc,
            &mut self.syscfg,
            &mut self.ethernet_mac,
            &mut self.ethernet_dma,
            ETH_ADDR,
        )
        .map(|device| device.into_interface());
        let mut iface = match ethernet_interface {
            Ok(iface) => iface,
            Err(e) => {
                println!("ethernet init failed: {:?}", e);
                return;
            }
        };

        let idle_stream = self.idle_stream;
        pin_mut!(idle_stream);

        let mut sockets = SocketSet::new(Vec::new());

        let dhcp_rx_buffer = UdpSocketBuffer::new([UdpPacketMetadata::EMPTY; 1], vec![0; 1500]);
        let dhcp_tx_buffer = UdpSocketBuffer::new([UdpPacketMetadata::EMPTY; 1], vec![0; 3000]);
        let mut dhcp = Dhcpv4Client::new(
            &mut sockets,
            dhcp_rx_buffer,
            dhcp_tx_buffer,
            Instant::from_millis(system_clock::ms() as i64),
        ).expect("could not bind udp socket for dhcp");
        let mut prev_ip_addr = iface.ipv4_addr().unwrap();

        // handle new ethernet packets
        loop {
            await!(idle_stream.next());
            let timestamp = Instant::from_millis(system_clock::ms() as i64);
            match iface.poll(&mut sockets, timestamp) {
                Err(::smoltcp::Error::Exhausted) => {
                    continue;
                }
                Err(::smoltcp::Error::Unrecognized) => print!("U"),
                Err(e) => println!("Network error: {:?}", e),
                Ok(socket_changed) => {
                    if socket_changed {
                        for mut socket in sockets.iter_mut() {
                            Self::poll_socket(&mut socket).expect("socket poll failed");
                        }
                    }
                }
            }

            let config = dhcp.poll(&mut iface, &mut sockets, timestamp)
                .unwrap_or_else(|e| {println!("DHCP: {:?}", e); None });
            let ip_addr = iface.ipv4_addr().unwrap();
            if ip_addr != prev_ip_addr {
                println!("\nAssigned a new IPv4 address: {}", ip_addr);
                iface.routes_mut().update(|routes_map| {
                    routes_map
                        .get(&IpCidr::new(Ipv4Address::UNSPECIFIED.into(), 0))
                        .map(|default_route| {
                            println!("Default gateway: {}", default_route.via_router);
                        });
                });
                for dns_server in config.iter().flat_map(|c| c.dns_servers.iter()).filter_map(|x| x.as_ref()) {
                    println!("DNS servers: {}", dns_server);
                }

                // TODO delete old sockets

                // add new sockets
                let endpoint = IpEndpoint::new(ip_addr.into(), 15);

                let udp_rx_buffer =
                    UdpSocketBuffer::new(vec![UdpPacketMetadata::EMPTY; 3], vec![0u8; 256]);
                let udp_tx_buffer =
                    UdpSocketBuffer::new(vec![UdpPacketMetadata::EMPTY; 1], vec![0u8; 128]);
                let mut example_udp_socket = UdpSocket::new(udp_rx_buffer, udp_tx_buffer);
                example_udp_socket.bind(endpoint).unwrap();
                sockets.add(example_udp_socket);

                let tcp_rx_buffer = TcpSocketBuffer::new(vec![0; ethernet::MTU]);
                let tcp_tx_buffer = TcpSocketBuffer::new(vec![0; ethernet::MTU]);
                let mut example_tcp_socket = TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer);
                example_tcp_socket.listen(endpoint).unwrap();
                sockets.add(example_tcp_socket);

                prev_ip_addr = ip_addr;
            }
            let mut timeout = dhcp.next_poll(timestamp);
            iface
                .poll_delay(&sockets, timestamp)
                .map(|sockets_timeout| timeout = sockets_timeout);
            // TODO await next interrupt
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
                smoltcp::dhcp::UDP_CLIENT_PORT => {}, // dhcp packet
                _ => unreachable!(),
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
                _ => unreachable!(),
            },
            _ => {}
        }
        Ok(())
    }
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

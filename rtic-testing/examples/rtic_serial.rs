#![deny(unsafe_code)]
#![no_main]
#![no_std]

use atsamd_hal::common::usb::usb_device::device::{UsbDevice, UsbDeviceBuilder, UsbVidPid};
use atsamd_hal::common::usb::usb_device::UsbError;
use atsamd_hal::common::usb::UsbBus;
use atsamd_hal::target_device::Interrupt;
use itsybitsy_m4::timer::TimerCounter3;
use itsybitsy_m4::usb::usb_device::bus::UsbBusAllocator;
use itsybitsy_m4::{
    clock::GenericClockController,
    dotstar_bitbang,
    gpio::v2::PA22,
    gpio::{Input, Output, Pa27, Pb2, Pb3, Pin, PullUp, PushPull},
    prelude::*,
    timer::{SpinTimer, TimerCounter, TimerCounter2},
    uart,
};
use panic_halt as _;
use smart_leds::{SmartLedsWrite, RGB8};
use usbd_serial::{DefaultBufferStore, SerialPort, USB_CLASS_CDC};
// use itsybitsy_m4::pac::Interrupt;

// I don't see a way to avoid writing this out since the Resources struct in an rtic app cannot
// be monomorphized (no generics) and we don't have an allocator to use Box<dyn SmartLedsWrite>.
type DotStar = apa102_spi::Apa102<
    bitbang_hal::spi::SPI<
        Pa27<Input<PullUp>>,
        Pb3<Output<PushPull>>,
        Pb2<Output<PushPull>>,
        TimerCounter3,
    >,
>;

#[rtic::app(device = itsybitsy_m4::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        timer: TimerCounter2,
        // led: DotStar,
        led: Pin<PA22, Output<PushPull>>,
        // led: itsybitsy_m4::gpio,
        usb_bus: &'static UsbBusAllocator<UsbBus>,
        usb_serial: SerialPort<'static, UsbBus>,
        usb_serial2: SerialPort<'static, UsbBus>,
        usb_device: UsbDevice<'static, UsbBus>,
    }

    #[init]
    fn init(c: init::Context) -> init::LateResources {
        static mut USB_ALLOCATOR: Option<UsbBusAllocator<UsbBus>> = None;

        let mut peripherals = c.device; // This mutability conversion is safe
        let mut clocks = GenericClockController::with_internal_32kosc(
            peripherals.GCLK,
            &mut peripherals.MCLK,
            &mut peripherals.OSC32KCTRL,
            &mut peripherals.OSCCTRL,
            &mut peripherals.NVMCTRL,
        );

        let gclk0 = clocks.gclk0();
        let timer_clock = clocks.tc2_tc3(&gclk0).unwrap();
        let mut timer = TimerCounter::tc2_(&timer_clock, peripherals.TC2, &mut peripherals.MCLK);

        timer.start(2.hz());
        timer.enable_interrupt(); // TODO: Is this necessary with rtic?

        let mut pins = itsybitsy_m4::Pins::new(peripherals.PORT);
        let mut red_led = pins.d13.into_open_drain_output(&mut pins.port);
        // pins.GPIO13;
        // let led = dotstar_bitbang(pins.dotstar, &mut pins.port, SpinTimer::new(12));
        // let led = dotstar_bitbang(pins.dotstar, &mut pins.port, timer2);

        let dotstar = itsybitsy_m4::pins::Dotstar {
            ci: pins.dotstar_ci,
            di: pins.dotstar_di,
            nc: pins.dotstar_nc,
        };

        // static mut USB_ALLOCATOR: Option<bus::UsbBusAllocator<UsbBusType>> = None;

        // let a = uart(
        //     pins.uart,
        //     &mut clocks,
        //     115200.hz(),
        //     peripherals.SERCOM3,
        //     &mut peripherals.MCLK,
        //     &mut pins.port,
        // );
        // dbgprint!(
        //     "\n\n\n\n~========== STARTING {:?} ==========~\n",
        //     itsybitsy_m4::serial_number()
        // );
        //
        // let rstc = &peripherals.RSTC;
        // dbgprint!("Last reset was from {:?}\n", itsybitsy_m4::reset_cause(rstc));

        let usb = itsybitsy_m4::pins::USB {
            dm: pins.usb_dm,
            dp: pins.usb_dp,
        };

        *USB_ALLOCATOR = Some(usb.usb_allocator(
            peripherals.USB,
            &mut clocks,
            &mut peripherals.MCLK,
            &mut pins.port,
        ));

        // TODO: Allocating two SerialPorts technically compiles and runs, but Linux
        //  isn't happy about some device descriptors and refuses to work with it
        let usb_allocator = USB_ALLOCATOR.as_ref().unwrap();
        let mut usb_serial = SerialPort::new(usb_allocator);
        let mut usb_serial2 = SerialPort::new(usb_allocator);

        let mut usb_device = UsbDeviceBuilder::new(usb_allocator, UsbVidPid(0x16c0, 0x27dd))
            .manufacturer("Fake Company")
            .product("Suspicious Serial Port")
            .serial_number("TEST")
            .device_class(USB_CLASS_CDC)
            .composite_with_iads()
            .build();

        let mut rgb = dotstar_bitbang(dotstar, &mut pins.port, SpinTimer::new(12));
        let off: [RGB8; 1] = [RGB8 { r: 0, g: 0, b: 0 }];
        rgb.write(off.iter().cloned()).unwrap();

        // unsafe {
        //     USB_SERIAL = Some(SerialPort::new(&bus_allocator));
        //     USB_BUS = Some(
        //         UsbDeviceBuilder::new(&bus_allocator, UsbVidPid(0x16c0, 0x27dd))
        //             .manufacturer("Fake company")
        //             .product("Serial port")
        //             .serial_number("TEST")
        //             .device_class(USB_CLASS_CDC)
        //             .build(),
        //     );
        // }

        init::LateResources {
            timer,
            led: red_led,
            usb_bus: usb_allocator,
            usb_serial,
            usb_serial2,
            usb_device,
        }
    }

    #[task(binds = USB_OTHER, resources = [usb_device, usb_serial, usb_serial2])]
    fn usb_other(cx: usb_other::Context) {
        usb_poll(
            cx.resources.usb_device,
            cx.resources.usb_serial,
            cx.resources.usb_serial2,
        );
    }

    #[task(binds = USB_TRCPT0, resources = [usb_device, usb_serial, usb_serial2])]
    fn usb_trcpt0(cx: usb_trcpt0::Context) {
        usb_poll(
            cx.resources.usb_device,
            cx.resources.usb_serial,
            cx.resources.usb_serial2,
        );
    }

    #[task(binds = USB_TRCPT1, resources = [usb_device, usb_serial, usb_serial2])]
    fn usb_trcpt1(cx: usb_trcpt1::Context) {
        usb_poll(
            cx.resources.usb_device,
            cx.resources.usb_serial,
            cx.resources.usb_serial2,
        );
    }

    #[task(binds = TC2, resources = [timer, led, usb_serial])]
    fn tc2(c: tc2::Context) {
        static mut EVEN: bool = true;

        if !c.resources.timer.wait().is_ok() {
            return;
        }

        c.resources.usb_serial.write(b"test\r\n").ok();
        // write!(c.resources.usb_serial, "test");

        // let color = [if *EVEN {
        //     // RGB8 { r: 60, g: 60, b: 0 }
        //     RGB8 { r: 255, g: 255, b: 255 }
        // } else {
        //     // RGB8 { r: 0, g: 60, b: 60 }
        //     RGB8 { r: 0, g: 0, b: 0 }
        // }];

        // c.resources.led.write(color.iter().cloned()).unwrap();
        if *EVEN {
            c.resources.led.set_high().unwrap();
        } else {
            c.resources.led.set_low().unwrap();
        }
        *EVEN = !*EVEN;
    }
};

// Throw away incoming data
fn usb_poll<B: usb_device::bus::UsbBus>(
    usb_dev: &mut UsbDevice<'static, B>,
    serial: &mut SerialPort<'static, B>,
    serial2: &mut SerialPort<'static, B>,
) {
    let serial_data = usb_dev.poll(&mut [serial]);
    let serial2_data = usb_dev.poll(&mut [serial2]);

    // if !usb_dev.poll(&mut [serial]) {
    //     return;
    // }
    if serial_data {
        let mut buf = [0; 10];
        match serial.read(&mut buf) {
            Ok(_) => {}
            Err(UsbError::WouldBlock) => {}
            e => panic!("USB read error: {:?}", e),
        }
    }

    if serial2_data {
        let mut buf = [0; 10];
        match serial2.read(&mut buf) {
            Ok(_) => {}
            Err(UsbError::WouldBlock) => {}
            e => panic!("USB read error: {:?}", e),
        }
    }
}

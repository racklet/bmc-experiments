#![deny(unsafe_code)]
#![no_main]
#![no_std]

use panic_probe as _;

#[rtic::app(device = itsybitsy_m4::pac, peripherals = true)]
mod app {
    use atsamd_hal::common::clock::GenericClockController;
    use atsamd_hal::common::gpio::*;
    use atsamd_hal::common::prelude::*;
    use atsamd_hal::common::timer::{SpinTimer, TimerCounter, TimerCounter2};
    use atsamd_hal::common::usb::UsbBus;
    use itsybitsy_m4::dotstar_bitbang;
    use rtt_target::{rprintln, rtt_init_print};
    use smart_leds::RGB8;
    use usb_device::bus::UsbBusAllocator;
    use usb_device::device::{UsbDevice, UsbDeviceBuilder, UsbVidPid};
    use usb_device::UsbError;
    use usbd_serial::SerialPort;
    use smart_leds::SmartLedsWrite;
    use rtic::Mutex;

    #[shared]
    struct Shared {
        usb_device: UsbDevice<'static, UsbBus>,
        usb_serial: SerialPort<'static, UsbBus>,
        usb_serial2: SerialPort<'static, UsbBus>,
    }

    #[local]
    struct Local {
        timer: TimerCounter2,
        led: Pa22<Output<PushPull>>,
        // usb_allocator: &'static UsbBusAllocator<UsbBus>,
    }

    #[init(local = [usb_allocator: Option<UsbBusAllocator<UsbBus>> = None])]
    fn init(c: init::Context) -> (Shared, Local, init::Monotonics) {
        // static mut USB_ALLOCATOR: Option<UsbBusAllocator<UsbBus>> = None;

        let mut device = c.device; // This mutability conversion is safe
        let mut clocks = GenericClockController::with_internal_32kosc(
            device.GCLK,
            &mut device.MCLK,
            &mut device.OSC32KCTRL,
            &mut device.OSCCTRL,
            &mut device.NVMCTRL,
        );

        let gclk0 = clocks.gclk0();
        let timer_clock = clocks.tc2_tc3(&gclk0).unwrap();
        let mut timer = TimerCounter::tc2_(&timer_clock, device.TC2, &mut device.MCLK);

        timer.start(2.hz());
        timer.enable_interrupt();

        let mut pins = itsybitsy_m4::Pins::new(device.PORT);
        let mut led = pins.d13.into_open_drain_output(&mut pins.port);

        let dotstar = itsybitsy_m4::pins::Dotstar {
            ci: pins.dotstar_ci,
            di: pins.dotstar_di,
            nc: pins.dotstar_nc,
        };

        let usb = itsybitsy_m4::pins::USB {
            dm: pins.usb_dm,
            dp: pins.usb_dp,
        };

        /*
        Bluepill Serial Monster comparison notes:
        - 16 bit words (bMaxPacketSize0 and serial port wMaxPacketSize) vs. our 8
        - uses composite_with_iads()
        - A CDC-ACM serial port is a pair of two interfaces: Communications and CDC Data
        - The serial monster simply exposes three pairs (6 interfaces)
         */

        // TODO: Allocating two SerialPorts technically compiles and runs, but Linux
        //  isn't happy about some device descriptors and refuses to work with it:
        /*
           usb 3-4: new full-speed USB device number 41 using xhci_hcd
           usb 3-4: unable to read config index 0 descriptor/start: -32
           usb 3-4: chopping to 0 config(s)
           usb 3-4: can't read configurations, error -32
           usb usb3-port4: unable to enumerate USB device
        */

        let a = Some(13);
        let b = a.as_ref().unwrap();

        *c.local.usb_allocator = Some(usb.usb_allocator(device.USB, &mut clocks, &mut device.MCLK, &mut pins.port));

        let mut usb_a = c.local.usb_allocator.as_ref().unwrap();

        let mut usb_serial = SerialPort::new(usb_a);
        let mut usb_serial2 = SerialPort::new(usb_a);

        let mut usb_device = UsbDeviceBuilder::new(usb_a, UsbVidPid(0x16c0, 0x27dd))
            .manufacturer("Racklet")
            .product("Rust ATSAMD Serial Hub")
            .serial_number("0000")
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

        (
            Shared {
                usb_device,
                usb_serial,
                usb_serial2,
            },
            Local {
                timer,
                led,
                // usb_allocator: usb_a,
            },
            init::Monotonics(),
        )
    }

    #[task(binds = USB_OTHER, shared = [usb_device, usb_serial, usb_serial2])]
    fn usb_other(cx: usb_other::Context) {
        // usb_poll(
        //     cx.shared.usb_device,
        //     cx.shared.usb_serial,
        //     cx.shared.usb_serial2,
        // );
    }

    #[task(binds = USB_TRCPT0, shared = [usb_device, usb_serial, usb_serial2])]
    fn usb_trcpt0(cx: usb_trcpt0::Context) {
        // usb_poll(
        //     &mut cx.shared.usb_device,
        //     &mut cx.shared.usb_serial,
        //     &mut cx.shared.usb_serial2,
        // );
    }

    #[task(binds = USB_TRCPT1, shared = [usb_device, usb_serial, usb_serial2])]
    fn usb_trcpt1(cx: usb_trcpt1::Context) {
        // usb_poll(
        //     &mut cx.shared.usb_device,
        //     &mut cx.shared.usb_serial,
        //     &mut cx.shared.usb_serial2,
        // );
    }

    #[task(binds = TC2, local = [timer, led], shared = [usb_serial, usb_serial2])]
    fn tc2(c: tc2::Context) {
        if !c.local.timer.wait().is_ok() {
            return;
        }

        // c.shared.usb_serial.write(b"Hello, World!\r\n").ok();
        // c.shared.usb_serial2.write(b"Another world!\r\n").ok();

        c.local.led.toggle();
    }

    use rtic::mutex_prelude::*;

    // Throw away incoming data
    fn usb_poll(
        usb_dev: &mut shared_resources::usb_device,
        serial: &mut shared_resources::usb_serial,
        serial2: &mut shared_resources::usb_serial2,
    ) {
        (usb_dev, serial, serial2).lock(|d, s, s2| {
            if d.poll(&mut [s, s2]) {
                let mut buf = [0; 10]; // Throwaway buffer
                match s.read(&mut buf) {
                    Ok(_) => {}
                    Err(UsbError::WouldBlock) => {} // No bytes available for reading
                    e => panic!("USB read error: {:?}", e),
                }
                match s2.read(&mut buf) {
                    Ok(_) => {}
                    Err(UsbError::WouldBlock) => {} // No bytes available for reading
                    e => panic!("USB read error: {:?}", e),
                }
            }
        });
    }
}

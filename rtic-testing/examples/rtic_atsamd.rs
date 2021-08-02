#![deny(unsafe_code)]
#![no_main]
#![no_std]

use atsamd_hal::target_device::Interrupt;
use itsybitsy_m4::timer::TimerCounter3;
use itsybitsy_m4::{
    clock::GenericClockController,
    dotstar_bitbang,
    gpio::v2::PA22,
    gpio::{Input, Output, Pa27, Pb2, Pb3, Pin, PullUp, PushPull},
    prelude::*, // Required for for example timer.start()
    timer::{SpinTimer, TimerCounter, TimerCounter2},
};
use panic_halt as _;
use smart_leds::{SmartLedsWrite, RGB8};
// use itsybitsy_m4::pac::Interrupt;

// I don't see a way to avoid writing this out since the Resources struct in an rtic app cannot
// be monomorphized (no generics) and we don't have an allocator to use Box<dyn SmartLedsWrite>.
// type DotStar = apa102_spi::Apa102<
//     bitbang_hal::spi::SPI<Pa27<Input<PullUp>>, Pb3<Output<PushPull>>, Pb2<Output<PushPull>>, SpinTimer>>;
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
    }

    #[init]
    fn init(c: init::Context) -> init::LateResources {
        let mut device = c.device; // This mutability conversion is safe
                                   // let mut clocks = GenericClockController::with_external_32kosc(
        let mut clocks = GenericClockController::with_internal_32kosc(
            device.GCLK,
            &mut device.MCLK,
            &mut device.OSC32KCTRL,
            &mut device.OSCCTRL,
            &mut device.NVMCTRL,
        );
        //
        let gclk0 = clocks.gclk0();
        let timer_clock = clocks.tc2_tc3(&gclk0).unwrap();
        let mut timer = TimerCounter::tc2_(&timer_clock, device.TC2, &mut device.MCLK);
        // // let mut timer2 = TimerCounter::tc3_(&timer_clock, device.TC3, &mut device.MCLK);
        //
        // timer.start(100.hz());

        timer.start(2.hz());
        timer.enable_interrupt();

        // timer2.start(10.hz());
        // timer2.enable_interrupt();

        let mut pins = itsybitsy_m4::Pins::new(device.PORT);
        let mut red_led = pins.d13.into_open_drain_output(&mut pins.port);
        // pins.GPIO13;
        // let led = dotstar_bitbang(pins.dotstar, &mut pins.port, SpinTimer::new(12));
        // let led = dotstar_bitbang(pins.dotstar, &mut pins.port, timer2);

        let dotstar = itsybitsy_m4::pins::Dotstar {
            ci: pins.dotstar_ci,
            di: pins.dotstar_di,
            nc: pins.dotstar_nc,
        };

        let mut rgb = dotstar_bitbang(dotstar, &mut pins.port, SpinTimer::new(12));
        let off: [RGB8; 1] = [RGB8 { r: 0, g: 0, b: 0 }];
        rgb.write(off.iter().cloned()).unwrap();

        // rtic::pend(Interrupt::TC2);
        // rtic::pend(Interrupt::TC3);

        init::LateResources {
            timer,
            led: red_led,
        }
    }

    #[task(binds = TC2, resources = [timer, led])]
    fn tc2(c: tc2::Context) {
        static mut EVEN: bool = true;

        if !c.resources.timer.wait().is_ok() {
            return;
        }

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

#![deny(unsafe_code)]
#![no_main]
#![no_std]

use panic_probe as _;

#[rtic::app(device = itsybitsy_m4::pac, peripherals = true)]
mod app {
    use atsamd_hal::clock::GenericClockController;
    use atsamd_hal::common::gpio::*;
    use atsamd_hal::common::prelude::*;
    use atsamd_hal::common::timer::{SpinTimer, TimerCounter, TimerCounter2, TimerCounter3};
    use itsybitsy_m4::dotstar_bitbang;
    use rtt_target::{rprintln, rtt_init_print};
    use smart_leds::{SmartLedsWrite, RGB8};

    type LED = Pa22<Output<PushPull>>;
    type DotStar = apa102_spi::Apa102<
        bitbang_hal::spi::SPI<
            Pa27<Input<PullUp>>,
            Pb3<Output<PushPull>>,
            Pb2<Output<PushPull>>,
            TimerCounter3,
        >,
    >;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        timer: TimerCounter2,
        // dotstar: DotStar,
        led: LED,
        state: bool,
    }

    #[init]
    fn init(c: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();

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
        // let mut timer2 = TimerCounter::tc3_(&timer_clock, device.TC3, &mut device.MCLK);

        timer.start(2.hz());
        timer.enable_interrupt();

        // timer2.start(10.hz());
        // timer2.enable_interrupt();

        let mut pins = itsybitsy_m4::Pins::new(device.PORT);
        let mut led = pins.d13.into_open_drain_output(&mut pins.port);
        // let dotstar = dotstar_bitbang(pins.dotstar, &mut pins.port, timer2);

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

        (
            Shared {},
            Local {
                timer,
                led,
                state: false,
            },
            init::Monotonics(),
        )
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        let mut counter: u32 = 0;

        loop {
            if counter % 1000 == 0 {
                rprintln!("Idling...");
            }

            counter += 1;
        }
    }

    #[task(binds = TC2, local = [timer, led, state])]
    fn tc2(c: tc2::Context) {
        if !c.local.timer.wait().is_ok() {
            return;
        }

        rprintln!("Hello, world!");

        // c.local.led.toggle();
        *c.local.state = !*c.local.state;
        if *c.local.state {
            c.local.led.set_high().unwrap();
        } else {
            c.local.led.set_low().unwrap();
        }
    }
}

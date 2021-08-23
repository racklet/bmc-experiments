#![no_std]
#![no_main]

use atsamd_hal::common::timer::SpinTimer;
use cortex_m::peripheral::Peripherals as CMP;
use cortex_m::peripheral::SYST;
use itsybitsy_m4::clock::GenericClockController;
use itsybitsy_m4::delay::Delay;
use itsybitsy_m4::pac::Peripherals;
use itsybitsy_m4::prelude::*;
use itsybitsy_m4::{dotstar_bitbang, entry};
use panic_probe as _;
use rtt_target::{rprintln, rtt_init_print};
use smart_leds::SmartLedsWrite;
use smart_leds::RGB8;

#[entry]
fn main() -> ! {
    rtt_init_print!();
    // hprintln!("Hello, semihosting!").unwrap();

    // exit QEMU
    // NOTE do not run this on hardware; it can corrupt OpenOCD state
    // debug::exit(debug::EXIT_SUCCESS);

    let mut cmp = CMP::take().unwrap();
    let mut peripherals = Peripherals::take().unwrap();
    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.MCLK,
        &mut peripherals.OSC32KCTRL,
        &mut peripherals.OSCCTRL,
        &mut peripherals.NVMCTRL,
    );

    let mut pins = itsybitsy_m4::Pins::new(peripherals.PORT);
    let mut delay = Delay::new(cmp.SYST, &mut clocks);

    let dotstar = itsybitsy_m4::pins::Dotstar {
        ci: pins.dotstar_ci,
        di: pins.dotstar_di,
        nc: pins.dotstar_nc,
    };

    let mut rgb = dotstar_bitbang(dotstar, &mut pins.port, SpinTimer::new(12));
    let off: [RGB8; 1] = [RGB8 { r: 0, g: 0, b: 0 }];
    let on: [RGB8; 1] = [RGB8 { r: 1, g: 1, b: 1 }];

    let a: Option<u8> = None;

    loop {
        // a.unwrap(); // Uncomment to test stack backtrace output
        rprintln!("Hello, world!");
        rgb.write(on.iter().cloned()).unwrap();
        delay.delay_ms(500u32);
        rgb.write(off.iter().cloned()).unwrap();
        delay.delay_ms(500u32);
    }
}

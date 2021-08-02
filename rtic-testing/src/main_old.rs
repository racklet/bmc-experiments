#![no_std]
#![no_main]

extern crate itsybitsy_m4 as hal;
extern crate panic_halt;

use hal::clock::GenericClockController;
use hal::delay::Delay;
use hal::entry;
use hal::pac::{CorePeripherals, Peripherals};
use hal::prelude::*;
use hal::timer::SpinTimer;
use hal::watchdog::{Watchdog, WatchdogTimeout};
use smart_leds::{hsv::RGB8, SmartLedsWrite};

use cortex_m_semihosting::hprintln;

#[entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
    let core = CorePeripherals::take().unwrap();
    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.MCLK,
        &mut peripherals.OSC32KCTRL,
        &mut peripherals.OSCCTRL,
        &mut peripherals.NVMCTRL,
    );
    let mut delay = Delay::new(core.SYST, &mut clocks);
    delay.delay_ms(400u16);

    let mut pins = hal::Pins::new(peripherals.PORT);
    let mut red_led = pins.d13.into_open_drain_output(&mut pins.port);

    let dotstar = hal::pins::Dotstar {
        ci: pins.dotstar_ci,
        di: pins.dotstar_di,
        nc: pins.dotstar_nc,
    };

    let mut rgb = hal::dotstar_bitbang(dotstar, &mut pins.port, SpinTimer::new(12));
    let off: [RGB8; 1] = [RGB8 { r: 0, g: 0, b: 0 }];
    rgb.write(off.iter().cloned()).unwrap();

    let mut wdt = Watchdog::new(peripherals.WDT);
    wdt.start(WatchdogTimeout::Cycles2K as u8);
    red_led.set_high().unwrap();

    loop {
        delay.delay_ms(1000u16);
        wdt.feed();
        red_led.set_high().unwrap();
        hprintln!("Hello, world!").unwrap();
        delay.delay_ms(1000u16);
        wdt.feed();
        red_led.set_low().unwrap();
    }
}

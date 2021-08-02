#![no_main]
#![no_std]

use core::{
    convert::TryFrom,
    mem,
    ops::RangeInclusive,
    panic::PanicInfo,
    ptr::{read_volatile, write_volatile},
    str::from_utf8_unchecked,
    sync::atomic::{self, Ordering},
};
use cortex_m::{asm::*, interrupt};
use itsybitsy_m4::{
    prelude::*,
    time::Hertz,
    timer::TimerCounter2, // Replacement for CountDownTimer<TIM2>
    usb::UsbBus,
};
use rtic::app;
// use stm32f1xx_hal::{
//     prelude::*,
//     time::Hertz,
//     usb::{
//         Peripheral,
//         UsbBus,
//         UsbBusType,
//     },
//     timer::{
//         CountDownTimer,
//         Timer,
//         Event,
//     },
//     pac::{
//         FLASH,
//         TIM2,
//     },
// };
use usb_device::{
    bus,
    device::{UsbDevice, UsbDeviceBuilder, UsbVidPid},
};
//use usb_device::prelude::*;
use itm_logger::*;
use usbd_mass_storage::USB_CLASS_MSC;
use usbd_scsi::{BlockDevice, BlockDeviceError, Scsi};
use usbd_serial::{CdcAcmClass, SerialPort, USB_CLASS_CDC};

// VID and PID are from dapboot bluepill bootloader
const USB_VID: u16 = 0x1209;
const USB_PID: u16 = 0xDB42;
//const USB_CLASS_MISCELLANEOUS: u8 =  0xEF;

const TICK_MS: u32 = 10;
const TICK_HZ: Hertz = Hertz(1000 / TICK_MS);

use atsamd_hal::common::timer::TimerCounter;
#[cfg(feature = "itm")]
use cortex_m::{iprintln, peripheral::ITM};

#[app(device = itsybitsy_m4::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        usb_dev: UsbDevice<'static, UsbBusType>,
        scsi: Scsi<'static, UsbBusType, GhostFat<FlashWrapper>>,
        tick_timer: TimerCounter2, // TODO: Replace with trait
    }

    #[init]
    fn init(mut cx: init::Context) -> init::LateResources {
        static mut USB_BUS: Option<bus::UsbBusAllocator<UsbBusType>> = None;

        // If caches are enabled, write operations to flash cause the core to hang because it
        // is very likely to attempt to load into the prefetch buffer while the write is happening
        // This can be proved by counting busy loops on the SR.BSY flag. With caches enabled this will
        // almost always get < 2 cycles. With caches disabled it's a much more relistic figure of
        // 350 cycles for a write and 150k cycles for a page erase.
        // However, since we're just busy looping while writing it doesn't really matter. Might be
        // worth disabling them if there was any useful work to be done in this time but for now,
        // leave them enabled.
        //cx.core.SCB.disable_icache();
        //cx.core.SCB.disable_dcache(&mut cx.core.CPUID);

        #[cfg(feature = "itm")]
        {
            update_tpiu_baudrate(8_000_000, ITM_BAUD_RATE).expect("Failed to reset TPIU baudrate");
            logger_init();
        }

        info!("ITM reset ok.");

        let mut flash = cx.device.FLASH.constrain();
        let mut rcc = cx.device.RCC.constrain();
        let bkp = rcc
            .bkp
            .constrain(cx.device.BKP, &mut rcc.apb1, &mut cx.device.PWR);
        let tim2 = cx.device.TIM2;

        let clocks = rcc
            .cfgr
            .use_hse(8.mhz())
            .sysclk(48.mhz())
            .pclk1(24.mhz())
            .freeze(&mut flash.acr);

        let a = itsybitsy_m4::pac::Peripherals::take().unwrap();

        #[cfg(feature = "itm")]
        {
            let sysclk: Hertz = clocks.sysclk().into();
            update_tpiu_baudrate(sysclk.0, ITM_BAUD_RATE).expect("Failed to reset TPIU baudrate");
        }

        assert!(clocks.usbclk_valid());

        let flash_kib = get_flash_kibi();
        info!("Flash: {} KiB", flash_kib);

        // This may not be 100% accurate. Cube hal has some random IFDEFs that don't even appear
        // to align with the core density.
        let page_size = if flash_kib > 128 { 2048 } else { 1024 };

        let flash_wrapper = FlashWrapper {
            page_size,
            page_buffer: [0; 2048],
            current_page: None,
            min_address: 0x08010000,
            max_address: 0x08000000 + flash_kib as u32 * 1024,
        };
        info!("Flash MAX: 0x{:X?}", flash_wrapper.max_address);

        let mut gpioa = cx.device.GPIOA.split(&mut rcc.apb2);

        // BluePill board has a pull-up resistor on the D+ line.
        // Pull the D+ pin down to send a RESET condition to the USB bus.
        // This forced reset is needed only for development, without it host
        // will not reset your device when you upload new firmware.
        let mut usb_dp = gpioa.pa12.into_push_pull_output(&mut gpioa.crh);
        usb_dp.set_low().unwrap();
        delay(clocks.sysclk().0 / 100);

        let usb_dm = gpioa.pa11;
        let usb_dp = usb_dp.into_floating_input(&mut gpioa.crh);

        let usb = Peripheral {
            usb: cx.device.USB,
            pin_dm: usb_dm,
            pin_dp: usb_dp,
        };

        // *USB_BUS = Some(UsbBus::new(usb));

        let mut tick_timer = Timer::tim2(tim2, &clocks, &mut rcc.apb1).start_count_down(TICK_HZ);
        tick_timer.listen(Event::Update);

        let ghost_fat = GhostFat::new(flash_wrapper, bkp);

        let scsi = Scsi::new(
            USB_BUS.as_ref().unwrap(),
            64,
            ghost_fat,
            "Fake Co.",
            "Fake product",
            "FK01",
        );

        let serial_number = get_serial_number();
        info!("Serial number: {}", serial_number);

        let usb_dev = UsbDeviceBuilder::new(USB_BUS.as_ref().unwrap(), UsbVidPid(USB_VID, USB_PID))
            .manufacturer("Fake company")
            .product("Serial port")
            .serial_number(serial_number)
            .self_powered(true)
            .device_class(USB_CLASS_MSC)
            .build();

        init::LateResources {
            usb_dev,
            scsi,
            tick_timer,
        }
    }

    #[task(binds = USB_HP_CAN_TX, resources = [usb_dev, scsi])]
    fn usb_tx(mut cx: usb_tx::Context) {
        usb_poll(&mut cx.resources.usb_dev, &mut cx.resources.scsi);
    }

    #[task(binds = USB_LP_CAN_RX0, resources = [usb_dev, scsi])]
    fn usb_rx0(mut cx: usb_rx0::Context) {
        usb_poll(&mut cx.resources.usb_dev, &mut cx.resources.scsi);
    }

    #[task(binds = TIM2, resources = [scsi, tick_timer])]
    fn tick(cx: tick::Context) {
        cx.resources.tick_timer.clear_update_interrupt_flag();

        cx.resources.scsi.block_device_mut().tick(TICK_MS);
    }
};

fn usb_poll<B: bus::UsbBus>(
    usb_dev: &mut UsbDevice<'static, B>,
    scsi: &mut Scsi<'static, B, GhostFat<FlashWrapper>>,
) {
    if !usb_dev.poll(&mut [scsi]) {
        return;
    }
}

#[panic_handler]
fn panic(#[cfg_attr(not(feature = "itm"), allow(unused_variables))] info: &PanicInfo) -> ! {
    interrupt::disable();

    #[cfg(feature = "itm")]
    {
        let itm = unsafe { &mut *ITM::ptr() };
        let stim = &mut itm.stim[0];

        iprintln!(stim, "{}", info);
    }

    loop {
        // add some side effect to prevent this from turning into a UDF instruction
        // see rust-lang/rust#28728 for details
        atomic::compiler_fence(Ordering::SeqCst)
    }
}

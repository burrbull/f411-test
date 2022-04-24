#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

// Halt on panic
use panic_halt as _; // panic handler

use core::fmt::Write;

use cortex_m_rt::entry;
use embedded_sdmmc::{Controller, SdMmcSpi, TimeSource, Timestamp, VolumeIdx};
use stm32f4xx_hal::{
    pac,
    prelude::*,
    serial::{config::Config, Serial},
    spi::{Mode, Phase, Polarity, Spi},
};

#[allow(clippy::empty_loop)]
#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().expect("cannot take peripherals");
    let _cp = cortex_m::Peripherals::take().expect("cannot take core peripherals");
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.freeze();

    let gpioa = dp.GPIOA.split();
    let gpiob = dp.GPIOB.split();

    let mut uart =
        Serial::new(dp.USART2, (gpioa.pa2, gpioa.pa3), Config::default(), &clocks).unwrap();

    let mode = Mode {
        polarity: Polarity::IdleLow,
        phase: Phase::CaptureOnFirstTransition,
    };

    let sdmmc_spi = Spi::new(
        dp.SPI2,
        (gpiob.pb13, gpiob.pb14, gpiob.pb15),
        mode,
        400.kHz(),
        &clocks,
    );

    let sdmmc_cs = gpiob.pb12.into_push_pull_output();
    let mut controller = Controller::new(SdMmcSpi::new(sdmmc_spi, sdmmc_cs), FakeTime {});

    writeln!(uart, "Init SD card...\r").unwrap();
    match controller.device().init() {
        Ok(_) => {
            write!(uart, "Card size... ").unwrap();
            match controller.device().card_size_bytes() {
                Ok(size) => writeln!(uart, "{}\r", size).unwrap(),
                Err(e) => writeln!(uart, "Err: {:?}", e).unwrap(),
            }
            writeln!(uart, "Volume 0:\r").unwrap();
            match controller.get_volume(VolumeIdx(0)) {
                Ok(volume) => {
                    let root_dir = controller.open_root_dir(&volume).unwrap();
                    writeln!(uart, "Listing root directory:\r").unwrap();
                    controller
                        .iterate_dir(&volume, &root_dir, |x| {
                            writeln!(uart, "Found: {:?}\r", x.name).unwrap();
                        })
                        .unwrap();
                    writeln!(uart, "End of listing\r").unwrap();
                }
                Err(e) => writeln!(uart, "Err: {:?}", e).unwrap(),
            }
        }
        Err(e) => writeln!(uart, "{:?}!", e).unwrap(),
    }

    loop {}
}

struct FakeTime;

impl TimeSource for FakeTime {
    fn get_timestamp(&self) -> Timestamp {
        Timestamp::from_calendar(1019, 11, 24, 3, 40, 31).unwrap()
    }
}

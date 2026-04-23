#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::String;
use hal::spi::master::Spi;
use core::fmt::Write;
use core::mem::MaybeUninit;
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::mono_font::{MonoTextStyle, MonoTextStyleBuilder};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Alignment, Text};
use esp_backtrace as _;
use esp_println::println;
use hal::adc::{AdcConfig, Attenuation, ADC, ADC2};
use hal::dma::DmaPriority;
use hal::gdma::Gdma;
use hal::gpio::NO_PIN;
use hal::prelude::_fugit_RateExtU32;
use hal::systimer::SystemTimer;
use hal::{
    clock::ClockControl, peripherals::Peripherals, prelude::*, timer::TimerGroup, Delay, Rtc,
    IO,
};
use t_display_s3_amoled::rm67162::Orientation;
use hal::spi::master::prelude::*;

#[global_allocator]

#[unsafe(export_name = "esp_app_desc")]
#[unsafe(link_section = ".rodata_desc")]
#[used]
pub static ESP_APP_DESC: esp_bootloader_esp_idf::EspAppDesc = esp_bootloader_esp_idf::EspAppDesc::new_internal(
    env!("CARGO_PKG_VERSION"),
    env!("CARGO_PKG_NAME"),
    "00:00:00",
    "2026-04-23",
    "esp-hal",
    0,
    u16::MAX,
    65536,
    0,
);

fn init_heap() {    
    const HEAP_SIZE: usize = 32 * 1024;

    unsafe {
    }
}

#[hal::entry]
fn main() -> ! {

    // Disable the RTC and TIMG watchdog timers


    // Set GPIO4 as an output, and set its state high initially.




    let sclk = io.pins.gpio47;
    let rst = io.pins.gpio17;
    let cs = io.pins.gpio6;

    let d0 = io.pins.gpio18;
    let d1 = io.pins.gpio7;
    let d2 = io.pins.gpio48;
    let d3 = io.pins.gpio5;



    let dma_channel = dma.channel0;

    // Descriptors should be sized as (BUFFERSIZE / 4092) * 3
    let mut descriptors = [0u32; 12];
    let spi = Spi::new_half_duplex(
        peripherals.SPI2, // use spi2 host
        75_u32.MHz(), // max 75MHz
        hal::spi::SpiMode::Mode0,
        &clocks)
        .with_pins(Some(sclk),Some(d0),Some(d1),Some(d2),Some(d3),NO_PIN)

    display
        .set_orientation(Orientation::LandscapeFlipped)


    // Create ADC instances
    let mut vbat_pin =


    Text::with_alignment(
        "Hello,\nRust World!",
        Point::new(300, 40),
        character_style,
        Alignment::Center,
    )
    .draw(&mut display)

    loop {
        // fps testing

        let vbat = (raw_val as f32 / 4095.0) * 3.3 * (100.0 + 100.0) / 100.0;

        /*
            let elapsed = now_ms() - started;
            core::write!(
                &mut s,
                "Frames: {}\nFPS: {:.1}",
                cnt,
                if elapsed > 0 {
                    cnt as f32 / (elapsed as f32 / 1000.0)
                } else {
                    0.0
                }
            )
        */
        Text::with_alignment(
            &s,
            Point::new(100, 40),
            MonoTextStyleBuilder::new()
                .background_color(Rgb565::BLACK)
                .text_color(Rgb565::CSS_BISQUE)
                .font(&FONT_10X20)
                .build(),
            Alignment::Center,
        )
        .draw(&mut display)
    }
}

fn now_ms() -> u64 {
    hal::time::Instant::now().duration_since_epoch().as_micros() * 1_000 / 1_000_000
}

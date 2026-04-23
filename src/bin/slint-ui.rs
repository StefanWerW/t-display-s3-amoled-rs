#![no_std]
#![no_main]

extern crate alloc;

use core::mem::MaybeUninit;

use alloc::boxed::Box;
use alloc::rc::Rc;

use embedded_graphics::pixelcolor::raw::RawU16;
use embedded_graphics::prelude::{DrawTarget, Point, Size};
use embedded_graphics::primitives::Rectangle;
use esp_backtrace as _;
use esp_println::println;
use hal::spi::master::Spi;
use hal::systimer::SystemTimer;
use hal::{
    clock::ClockControl, dma::DmaPriority, gdma::Gdma, gpio::NO_PIN, peripherals::Peripherals,
    prelude::*, timer::TimerGroup, Delay, Rtc, IO,
};
use hal::spi::master::prelude::*;

use slint::platform::software_renderer::{MinimalSoftwareWindow, Rgb565Pixel};
use slint::platform::{software_renderer as renderer, Platform};
use slint::PhysicalSize;

use t_display_s3_amoled::rm67162::dma::RM67162Dma;
use t_display_s3_amoled::rm67162::Orientation;

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


struct Backend {
    window: Rc<renderer::MinimalSoftwareWindow>,
}

impl Platform for Backend {
    fn create_window_adapter(
        &self,
    ) -> Result<alloc::rc::Rc<dyn slint::platform::WindowAdapter>, slint::PlatformError> {
        // Since on MCUs, there can be only one window, just return a clone of self.window.
        // We'll also use the same window in the event loop.
        Ok(self.window.clone())
    }

    fn duration_since_start(&self) -> core::time::Duration {
        core::time::Duration::from_millis(
            hal::time::Instant::now().duration_since_epoch().as_micros() * 1_000 / 1_000_000,
        )
    }

    // fn run_event_loop(&self) -> Result<(), slint::PlatformError>
    fn debug_log(&self, arguments: core::fmt::Arguments) {
    }
}

struct DisplayWrapper<'a, CS> {
    display: &'a mut RM67162Dma<'a,CS>,
    line_buffer: &'a mut [Rgb565Pixel; 536],
}

impl<CS> renderer::LineBufferProvider for &mut DisplayWrapper<'_, CS>
where
    CS: embedded_hal_1::digital::OutputPin,
{
    type TargetPixel = Rgb565Pixel;

    fn process_line(
        &mut self,
        line: usize,
        range: core::ops::Range<usize>,
        render_fn: impl FnOnce(&mut [Self::TargetPixel]),
    ) {

        let _ = self.display.fill_contiguous(
            &Rectangle::new(
                Point::new(range.start as _, line as _),
                Size::new(range.len() as _, 1),
            ),
            self.line_buffer[range.clone()]
                .iter()
                .map(|p| RawU16::new(p.0).into()),
    }
}

#[hal::entry]
fn main() -> ! {

    // Disable the RTC and TIMG watchdog timers

    // Set GPIO4 as an output, and set its state high initially.


    // Initialize the Delay peripheral, and use it to toggle the LED state in a
    // loop.


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
        // NO_PIN,       // Some(cs), NOTE: manually control cs
        75_u32.MHz(), // max 75MHz
        hal::spi::SpiMode::Mode0,
        &clocks)
        .with_pins(Some(sclk),Some(d0),Some(d1),Some(d2),Some(d3),NO_PIN)

    display
        .set_orientation(Orientation::LandscapeFlipped)


    slint::platform::set_platform(Box::new(Backend {
        window: window.clone(),
    }))


    let mut wrapper = DisplayWrapper {
        display: &mut display,
        line_buffer: &mut line_buffer,
    };

    let mut i = 0;
    loop {

        i += 1;
        if i > 100 {
            i = 0;
        }

        // Draw the scene if something needs to be drawn.
        window.draw_if_needed(|renderer| {

        if !window.has_active_animations() {
            // if no animation is running, wait for the next input event
        }

    }
}

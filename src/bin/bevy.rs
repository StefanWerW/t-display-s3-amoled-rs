#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::String;
use bevy::ecs::query;
use bevy::prelude::*;
use bevy_platform::time::Instant;
use core::fmt::Write;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU32, Ordering};
use embedded_graphics::framebuffer::{buffer_size, Framebuffer};
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::mono_font::{MonoTextStyle, MonoTextStyleBuilder};
use embedded_graphics::pixelcolor::raw::{LittleEndian, RawU16};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{
    Circle, PrimitiveStyle, Rectangle, StyledDimensions, StyledDrawable,
};
use embedded_graphics::text::{Alignment, Text};
use esp_backtrace as _;
use esp_println::println;
use t_display_s3_amoled::rm67162::dma::RM67162Dma;

use hal::{
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    spi::master::{Config as SpiConfig, Spi},
    spi::Mode,
    time::Rate,
};

static ELAPSED: AtomicU32 = AtomicU32::new(0);
fn elapsed_time() -> core::time::Duration {
    core::time::Duration::from_millis(ELAPSED.load(Ordering::Relaxed) as u64)
}

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

#[unsafe(export_name = "esp_app_desc")]
#[unsafe(link_section = ".rodata_desc")]
#[used]
pub static ESP_APP_DESC: esp_bootloader_esp_idf::EspAppDesc =
    esp_bootloader_esp_idf::EspAppDesc::new_internal(
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
    const HEAP_SIZE: usize = 5 * 32 * 1024;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

    unsafe {
        ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
    }
}

type MyDisplay = RM67162Dma<'static, Output<'static>, Output<'static>>;

struct DisplayResource {
    display: MyDisplay,
}

const LCD_H_RES: usize = 130;
const LCD_V_RES: usize = 240;
const LCD_BUFFER_SIZE: usize = LCD_H_RES * LCD_V_RES;

type MyFrameBuffer = Framebuffer<
    Rgb565,
    RawU16,
    LittleEndian,
    LCD_H_RES,
    LCD_V_RES,
    { buffer_size::<Rgb565>(LCD_H_RES, LCD_V_RES) },
>;

struct FramebufferResource {
    fb: MyFrameBuffer,
}

#[derive(Component)]
struct Ball;

#[derive(Component)]
struct Velocity {
    x: f32,
    y: f32,
}

fn ball_movement(mut query: Query<(&mut Transform, &Velocity)>) {
    for (mut transform, velocity) in query.iter_mut() {
        transform.translation.x += velocity.x;
        transform.translation.y += velocity.y;
    }
}

fn render_system(
    mut display: NonSendMut<DisplayResource>,
    mut fb_res: NonSendMut<FramebufferResource>,
    mut query: Query<(&Transform, Entity)>,
) {
    println!("render_system");

    fb_res.fb.clear(Rgb565::BLACK).unwrap();

    for (transform, _) in query.iter() {
        let circle = Circle::new(
            Point {
                x: transform.translation.x as i32,
                y: transform.translation.y as i32,
            },
            20,
        );

        println!("drawing circle at {:?}", circle.center());

        let draw_result = circle
            .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
            .draw(&mut fb_res.fb);
        match draw_result {
            Ok(_) => {}
            Err(e) => println!("draw_result: {:?}", e),
        };
    }

    fb_res.fb.as_image().draw(&mut display.display).unwrap();
}

fn init_world(mut commands: Commands) {
    commands.spawn((
        Ball,
        Transform::from_xyz(50.0, 50.0, 0.0),
        Velocity { x: 0.01, y: 0.01 },
    ));
}

#[hal::main]
fn main() -> ! {
    init_heap();
    let peripherals = hal::init(hal::Config::default());
    println!("Hello world!");

    let mut delay = Delay::new();

    let _led = Output::new(peripherals.GPIO38, Level::High, OutputConfig::default());
    println!("GPIO init OK");

    let sclk = peripherals.GPIO47;
    let mut rst = Output::new(peripherals.GPIO17, Level::High, OutputConfig::default());
    let cs = Output::new(peripherals.GPIO6, Level::High, OutputConfig::default());

    let d0 = peripherals.GPIO18; // MOSI
    let d1 = peripherals.GPIO7; // DC

    let dc = Output::new(d1, Level::High, OutputConfig::default());

    let dma_channel = peripherals.DMA_CH0;

    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = hal::dma_buffers!(32000, 32000);
    let dma_rx_buf = hal::dma::DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
    let dma_tx_buf = hal::dma::DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

    let spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(75))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(sclk)
    .with_mosi(d0)
    .with_dma(dma_channel)
    .with_buffers(dma_rx_buf, dma_tx_buf);

    let mut display = t_display_s3_amoled::rm67162::dma::RM67162Dma::new(spi, cs, dc);
    display.reset(&mut rst, &mut delay).unwrap();
    println!("reset display");
    display.init(&mut delay).unwrap();
    display
        .set_orientation(t_display_s3_amoled::rm67162::Orientation::LandscapeFlipped)
        .unwrap();

    println!("init display");

    unsafe { Instant::set_elapsed(elapsed_time) };

    display.clear(Rgb565::WHITE).unwrap();
    println!("screen init ok");

    let mut app = App::new();
    app.add_plugins((DefaultPlugins,))
        .insert_non_send_resource(DisplayResource { display })
        .insert_non_send_resource(FramebufferResource {
            fb: MyFrameBuffer::new(),
        })
        .add_systems(
            Update,
            (
                //ball_movement,
                render_system
            ),
        )
        .add_systems(Startup, init_world)
        .run();

    let mut loop_delay = Delay::new();

    loop {
        // fps testing
        // let mut s = String::new();

        // let elapsed = hal::time::Instant::now().duration_since_epoch().as_millis() - started;
        // core::write!(
        //     &mut s,
        //     "Frames: {}\nFPS: {:.1}",
        //     cnt,
        //     if elapsed > 0 {
        //         cnt as f32 / (elapsed as f32 / 1000.0)
        //     } else {
        //         0.0
        //     }
        // )
        // .unwrap();
        // Text::with_alignment(
        //     &s,
        //     Point::new(100, 40),
        //     MonoTextStyleBuilder::new()
        //         .background_color(Rgb565::BLACK)
        //         .text_color(Rgb565::CSS_BISQUE)
        //         .font(&FONT_10X20)
        //         .build(),
        //     Alignment::Center,
        // )
        // .draw(&mut display)
        // .unwrap();
        // cnt += 1;

        app.update();
        loop_delay.delay_millis(50u32);
    }
}

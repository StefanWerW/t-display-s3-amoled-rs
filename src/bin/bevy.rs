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
use embedded_graphics_framebuf::FrameBuf;
use esp_backtrace as _;
use esp_println::println;
use t_display_s3_amoled::heapbuffer::HeapBuffer;
use t_display_s3_amoled::rm67162::dma::RM67162Dma;

use esp_hal::{
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

// #[global_allocator]
// static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

esp_bootloader_esp_idf::esp_app_desc!();

// fn init_heap() {
//     const HEAP_SIZE: usize = 6 * 32 * 1024;
//     static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

//     unsafe {
//         ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
//     }
// }

type MyDisplay = RM67162Dma<'static, Output<'static>, Output<'static>>;

struct DisplayResource {
    display: MyDisplay,
}

const LCD_H_RES: usize = 536;
const LCD_V_RES: usize = 240;
const LCD_BUFFER_SIZE: usize = LCD_H_RES * LCD_V_RES;

type FbBuffer = HeapBuffer<Rgb565, LCD_BUFFER_SIZE>;
type MyFrameBuf = FrameBuf<Rgb565, FbBuffer>;

#[derive(Resource)]
struct FrameBufferResource {
    frame_buf: MyFrameBuf,
}

impl FrameBufferResource {
    fn new() -> Self {
        let fb_data: Box<[Rgb565; LCD_BUFFER_SIZE]> = Box::new([Rgb565::BLACK; LCD_BUFFER_SIZE]);
        let heap_buffer = HeapBuffer::new(fb_data);
        let frame_buf = MyFrameBuf::new(heap_buffer, LCD_H_RES, LCD_V_RES);
        Self { frame_buf }
    }
}

#[derive(Component)]
struct Ball {
    radius: u32,
    color: Rgb565,
}

#[derive(Component)]
struct Velocity {
    x: f32,
    y: f32,
}

const EPSILON: f32 = 0.1;

fn ball_movement(mut query: Query<(&mut Transform, &mut Velocity, &Ball)>) {
    for (mut transform, mut velocity, ball) in query.iter_mut() {
        transform.translation.x += velocity.x;
        transform.translation.y += velocity.y;

        if transform.translation.x > LCD_H_RES as f32 - ball.radius as f32 {
            velocity.x = -velocity.x;
            transform.translation.x = LCD_H_RES as f32 - ball.radius as f32 - EPSILON;
        }
        if transform.translation.x < ball.radius as f32 {
            velocity.x = -velocity.x;
            transform.translation.x = ball.radius as f32 + EPSILON;
        }
        if transform.translation.y > LCD_V_RES as f32 - ball.radius as f32 {
            velocity.y = -velocity.y;
            transform.translation.y = LCD_V_RES as f32 - ball.radius as f32 - EPSILON;
        }
        if transform.translation.y < ball.radius as f32 {
            velocity.y = -velocity.y;
            transform.translation.y = ball.radius as f32 + EPSILON;
        }
    }
}

fn render_system(
    mut display_res: NonSendMut<DisplayResource>,
    mut fb_res: ResMut<FrameBufferResource>,
    mut query: Query<(&Transform, &Ball)>,
) {
    // println!("render_system");

    fb_res.frame_buf.clear(Rgb565::BLACK).unwrap();

    for (transform, ball) in query.iter() {
        let circle = Circle::new(
            Point {
                x: transform.translation.x as i32,
                y: transform.translation.y as i32,
            },
            ball.radius,
        );

        // println!("drawing circle at {:?}", circle.center());

        let draw_result = circle
            .into_styled(PrimitiveStyle::with_fill(ball.color))
            .draw(&mut fb_res.frame_buf);
        match draw_result {
            Ok(_) => {}
            Err(e) => println!("draw_result: {:?}", e),
        };
    }

    let area = Rectangle::new(Point::zero(), fb_res.frame_buf.size());
    display_res
        .display
        .fill_contiguous(&area, fb_res.frame_buf.data.iter().copied())
        .unwrap();
}

fn init_world(mut commands: Commands) {
    commands.spawn((
        Ball {
            radius: 50,
            color: Rgb565::CSS_BEIGE,
        },
        Transform::from_xyz(200.0, 150.0, 0.0),
        Velocity { x: 1.0, y: 3.0 },
    ));

    commands.spawn((
        Ball {
            radius: 40,
            color: Rgb565::CSS_LIME_GREEN,
        },
        Transform::from_xyz(150.0, 100.0, 0.0),
        Velocity { x: -1.5, y: 2.3 },
    ));

    commands.spawn((
        Ball {
            radius: 30,
            color: Rgb565::CSS_ORANGE_RED,
        },
        Transform::from_xyz(100.0, 100.0, 0.0),
        Velocity { x: 2.2, y: -2.7 },
    ));

    commands.spawn((
        Ball {
            radius: 20,
            color: Rgb565::CSS_INDIAN_RED,
        },
        Transform::from_xyz(50.0, 400.0, 0.0),
        Velocity { x: -2.0, y: 2.0 },
    ));

    commands.spawn((
        Ball {
            radius: 10,
            color: Rgb565::CSS_BURLY_WOOD,
        },
        Transform::from_xyz(50.0, 50.0, 0.0),
        Velocity { x: 1.7, y: -3.1 },
    ));
}

#[esp_hal::main]
fn main() -> ! {
    //init_heap();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);
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

    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) =
        esp_hal::dma_buffers!(32000, 32000);
    let dma_rx_buf = esp_hal::dma::DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
    let dma_tx_buf = esp_hal::dma::DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

    let spi = esp_hal::spi::master::Spi::new(
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
        .insert_resource(FrameBufferResource::new())
        .add_systems(Update, (ball_movement, render_system))
        .add_systems(Startup, init_world);

    let mut loop_delay = Delay::new();

    let mut cnt = 0;
    let started = esp_hal::time::Instant::now()
        .duration_since_epoch()
        .as_millis();

    println!("loop start");

    loop {
        //fps testing
        let mut s = String::new();

        let elapsed = esp_hal::time::Instant::now()
            .duration_since_epoch()
            .as_millis()
            - started;
        core::write!(
            &mut s,
            "FPS: {:.1}",
            if elapsed > 0 {
                cnt as f32 / (elapsed as f32 / 1000.0)
            } else {
                0.0
            }
        )
        .unwrap();

        println!("{}", s);
        cnt += 1;

        app.update();
        //loop_delay.delay_millis(50u32);
    }
}

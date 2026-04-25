#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec; // Added for dynamically sizing our particle coordinate lists
use bevy::prelude::*;
use bevy_platform::time::Instant;
use core::fmt::Write;
use core::sync::atomic::{AtomicU32, Ordering};
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::mono_font::ascii::FONT_4X6;
use embedded_graphics::mono_font::{MonoTextStyle, MonoTextStyleBuilder};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, PrimitiveStyle, Rectangle, StyledDrawable};
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

esp_bootloader_esp_idf::esp_app_desc!();

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

// Global state to track animation time
#[derive(Resource)]
struct MorphState {
    start_time: u64,
}

// Our new Component replacing Ball and Velocity
#[derive(Component)]
struct Particle {
    start_pos: Vec2,
    end_pos: Vec2,
}

// ---------------------------------------------------------
// SYSTEM: Morph Animation (Linear Interpolation)
// ---------------------------------------------------------
fn morph_system(time_res: Res<MorphState>, mut query: Query<(&mut Transform, &Particle)>) {
    let now = esp_hal::time::Instant::now()
        .duration_since_epoch()
        .as_millis() as u64;
    let elapsed = now - time_res.start_time;

    // Cycle length: 10000ms (5 seconds forward, 5 seconds backward)
    // Change 10000 and 5000.0 to whatever duration you like!
    let cycle = (elapsed % 10000) as f32 / 5000.0;

    // Create a ping-pong value 't' that goes 0.0 -> 1.0 -> 0.0
    let t = if cycle > 1.0 { 2.0 - cycle } else { cycle };

    // Apply a smoothstep function (ease-in-out) so the particles slow down before settling
    let t_eased = t * t * (3.0 - 2.0 * t);

    // Apply the LERP math: Current = Start + t * (End - Start)
    for (mut transform, particle) in query.iter_mut() {
        transform.translation.x =
            particle.start_pos.x + (particle.end_pos.x - particle.start_pos.x) * t_eased;
        transform.translation.y =
            particle.start_pos.y + (particle.end_pos.y - particle.start_pos.y) * t_eased;
    }
}

// ---------------------------------------------------------
// SYSTEM: Render Particles
// ---------------------------------------------------------
fn render_system(
    mut display_res: NonSendMut<DisplayResource>,
    mut fb_res: ResMut<FrameBufferResource>,
    query: Query<&Transform, With<Particle>>,
) {
    fb_res.frame_buf.clear(Rgb565::BLACK).unwrap();

    for transform in query.iter() {
        // Draw a small 2-pixel radius circle for each particle
        let circle = Circle::new(
            Point {
                x: transform.translation.x as i32,
                y: transform.translation.y as i32,
            },
            2,
        );

        let _ = circle
            .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
            .draw(&mut fb_res.frame_buf);
    }

    let area = Rectangle::new(Point::zero(), fb_res.frame_buf.size());
    display_res
        .display
        .fill_contiguous(&area, fb_res.frame_buf.data.iter().copied())
        .unwrap();
}

// ---------------------------------------------------------
// STARTUP SYSTEM: Extract Coordinates and Spawn Entities
// ---------------------------------------------------------
fn init_world(mut commands: Commands, mut fb_res: ResMut<FrameBufferResource>) {
    let center = Point::new(LCD_H_RES as i32 / 2, LCD_V_RES as i32 / 2);
    let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);

    // 1. Draw "Mariana" and extract pixel locations
    fb_res.frame_buf.clear(Rgb565::BLACK).unwrap();
    Text::with_alignment("Mariana", center, text_style, Alignment::Center)
        .draw(&mut fb_res.frame_buf)
        .unwrap();

    let mut mariana_pts = Vec::new();
    for (i, color) in fb_res.frame_buf.data.iter().enumerate() {
        if *color != Rgb565::BLACK {
            mariana_pts.push(Vec2::new((i % LCD_H_RES) as f32, (i / LCD_H_RES) as f32));
        }
    }

    // 2. Draw "Stefan" and extract pixel locations
    fb_res.frame_buf.clear(Rgb565::BLACK).unwrap();
    Text::with_alignment("Stefan", center, text_style, Alignment::Center)
        .draw(&mut fb_res.frame_buf)
        .unwrap();

    let mut stefan_pts = Vec::new();
    for (i, color) in fb_res.frame_buf.data.iter().enumerate() {
        if *color != Rgb565::BLACK {
            stefan_pts.push(Vec2::new((i % LCD_H_RES) as f32, (i / LCD_H_RES) as f32));
        }
    }

    // Clear the buffer again so it's fresh for the render loop
    fb_res.frame_buf.clear(Rgb565::BLACK).unwrap();

    // 3. Match arrays and spawn particles
    let max_particles = mariana_pts.len().max(stefan_pts.len());
    if max_particles == 0 {
        return; // Failsafe in case nothing rendered
    }

    let scale = 2.5; // Change this to make the text as huge as you want!
    let cx = LCD_H_RES as f32 / 2.0;
    let cy = LCD_V_RES as f32 / 2.0;

    for i in 0..max_particles {
        let m_pt = mariana_pts[i % mariana_pts.len()];
        let s_pt = stefan_pts[i % stefan_pts.len()];

        // Scale the points outward from the center of the screen
        let start = Vec2::new(cx + (m_pt.x - cx) * scale, cy + (m_pt.y - cy) * scale);
        let end = Vec2::new(cx + (s_pt.x - cx) * scale, cy + (s_pt.y - cy) * scale);

        commands.spawn((
            Particle {
                start_pos: start,
                end_pos: end,
            },
            Transform::from_xyz(start.x, start.y, 0.0),
        ));
    }
}

// ---------------------------------------------------------
// MAIN LOOP
// ---------------------------------------------------------
#[esp_hal::main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);
    println!("Hello world!");

    let mut delay = Delay::new();
    let _led = Output::new(peripherals.GPIO38, Level::High, OutputConfig::default());

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
    display.init(&mut delay).unwrap();
    display
        .set_orientation(t_display_s3_amoled::rm67162::Orientation::LandscapeFlipped)
        .unwrap();

    unsafe { Instant::set_elapsed(elapsed_time) };
    display.clear(Rgb565::WHITE).unwrap();

    let started = esp_hal::time::Instant::now()
        .duration_since_epoch()
        .as_millis();

    let mut app = App::new();
    app.add_plugins((DefaultPlugins,))
        .insert_non_send_resource(DisplayResource { display })
        .insert_resource(FrameBufferResource::new())
        // Register the time resource so our morph system knows when to loop
        .insert_resource(MorphState {
            start_time: started as u64,
        })
        .add_systems(Startup, init_world)
        // Ensure Morphing happens before Rendering in the update cycle
        .add_systems(Update, (morph_system, render_system).chain());

    let mut cnt = 0;
    println!("loop start");

    loop {
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
    }
}

#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::String;
use core::fmt::Write;
use core::mem::MaybeUninit;
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::mono_font::{MonoTextStyle, MonoTextStyleBuilder};
use embedded_graphics::pixelcolor::{Rgb565, Rgb888};
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Alignment, Text};
use esp_backtrace as _;
use esp_println::println;

use hal::{
    delay::Delay,
    gpio::{Io, Level, Output, OutputConfig},
    spi::master::{Config as SpiConfig, Spi},
    spi::Mode,
    time::Rate,
};

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
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

    unsafe {
        ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

impl Vector2 {
    /// Create a new Vector2
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Calculate the dot product with another Vector2
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    /// Element-wise Sine
    pub fn sin(self) -> Self {
        Self {
            x: libm::sinf(self.x),
            y: libm::sinf(self.y),
        }
    }

    /// Element-wise Cosine
    pub fn cos(self) -> Self {
        Self {
            x: libm::cosf(self.x),
            y: libm::cosf(self.y),
        }
    }

    /// Element-wise Hyperbolic Tangent
    pub fn tanh(self) -> Self {
        Self {
            x: libm::tanhf(self.x),
            y: libm::tanhf(self.y),
        }
    }
}

// ---------------------------------------------------------
// Operator Overloads for Multiplication
// ---------------------------------------------------------

/// Scalar multiplication (Vector * f32)
impl core::ops::Mul<f32> for Vector2 {
    type Output = Self;

    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

/// Element-wise Vector multiplication (Vector * Vector)
impl core::ops::Mul<Vector2> for Vector2 {
    type Output = Self;

    fn mul(self, other: Vector2) -> Self {
        Self {
            x: self.x * other.x,
            y: self.y * other.y,
        }
    }
}

impl core::ops::Add<f32> for Vector2 {
    type Output = Self;

    fn add(self, scalar: f32) -> Self {
        Self {
            x: self.x + scalar,
            y: self.y + scalar,
        }
    }
}

impl core::ops::AddAssign<f32> for Vector2 {
    fn add_assign(&mut self, scalar: f32) {
        self.x += scalar;
        self.y += scalar;
    }
}

impl core::ops::Add<Vector2> for Vector2 {
    type Output = Self;

    fn add(self, other: Vector2) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl core::ops::AddAssign<Vector2> for Vector2 {
    fn add_assign(&mut self, other: Vector2) {
        self.x += other.x;
        self.y += other.y;
    }
}

impl core::ops::Sub<f32> for Vector2 {
    type Output = Self;

    fn sub(self, scalar: f32) -> Self {
        Self {
            x: self.x - scalar,
            y: self.y - scalar,
        }
    }
}

impl core::ops::SubAssign<f32> for Vector2 {
    fn sub_assign(&mut self, scalar: f32) {
        self.x -= scalar;
        self.y -= scalar;
    }
}

impl core::ops::Sub<Vector2> for Vector2 {
    type Output = Self;

    fn sub(self, other: Vector2) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl core::ops::SubAssign<Vector2> for Vector2 {
    fn sub_assign(&mut self, other: Vector2) {
        self.x -= other.x;
        self.y -= other.y;
    }
}

impl core::ops::Div<f32> for Vector2 {
    type Output = Self;

    fn div(self, scalar: f32) -> Self {
        Self {
            x: self.x / scalar,
            y: self.y / scalar,
        }
    }
}

impl core::ops::DivAssign<f32> for Vector2 {
    fn div_assign(&mut self, scalar: f32) {
        self.x /= scalar;
        self.y /= scalar;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Vector4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vector4 {
    /// Create a new Vector4
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub fn sin(self) -> Self {
        Self {
            x: libm::sinf(self.x),
            y: libm::sinf(self.y),
            z: libm::sinf(self.z),
            w: libm::sinf(self.w),
        }
    }

    /// Element-wise Exponential
    pub fn exp(self) -> Self {
        Self {
            x: libm::expf(self.x),
            y: libm::expf(self.y),
            z: libm::expf(self.z),
            w: libm::expf(self.w),
        }
    }

    /// Element-wise Hyperbolic Tangent
    pub fn tanh(self) -> Self {
        Self {
            x: libm::tanhf(self.x),
            y: libm::tanhf(self.y),
            z: libm::tanhf(self.z),
            w: libm::tanhf(self.w),
        }
    }
}

impl core::ops::Add<Vector4> for Vector4 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
            w: self.w + other.w,
        }
    }
}

impl core::ops::Mul<Vector4> for Vector4 {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Self {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z,
            w: self.w * other.w,
        }
    }
}

impl core::ops::Mul<f32> for Vector4 {
    type Output = Self;

    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
            w: self.w * scalar,
        }
    }
}

impl core::ops::Div<Vector4> for Vector4 {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        Self {
            x: self.x / other.x,
            y: self.y / other.y,
            z: self.z / other.z,
            w: self.w / other.w,
        }
    }
}

#[hal::main]
fn main() -> ! {
    init_heap();
    let peripherals = hal::init(hal::Config::default());
    println!("Hello world!");

    let mut delay = Delay::new();

    let mut led = Output::new(peripherals.GPIO38, Level::High, OutputConfig::default());
    println!("GPIO init OK");

    let sclk = peripherals.GPIO47;
    let mut rst = Output::new(peripherals.GPIO17, Level::High, OutputConfig::default());
    let mut cs = Output::new(peripherals.GPIO6, Level::High, OutputConfig::default());

    let d0 = peripherals.GPIO18; // MOSI
    let d1 = peripherals.GPIO7; // DC

    let dc = Output::new(d1, Level::High, OutputConfig::default());

    let spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(75))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(sclk)
    .with_mosi(d0);

    let mut display = t_display_s3_amoled::rm67162::RM67162::new(spi, cs, dc);
    display.reset(&mut rst, &mut delay).unwrap();
    println!("reset display");
    display.init(&mut delay).unwrap();
    display
        .set_orientation(t_display_s3_amoled::rm67162::Orientation::LandscapeFlipped)
        .unwrap();

    println!("init display");

    display.clear(Rgb565::WHITE).unwrap();
    println!("screen init ok");

    let mut cnt = 0;
    let started = hal::time::Instant::now().duration_since_epoch().as_millis();

    loop {
        // fps testing
        let mut s = String::new();

        let elapsed = hal::time::Instant::now().duration_since_epoch().as_millis() - started;
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
        .unwrap();

        /*
        vec2 p=(FC.xy*2.-r)/r.y,l,v=p*(1.-(l+=abs(.7-dot(p,p))))/.2;for(float i;i++<8.;o+=(sin(v.xyyx)+1.)*abs(v.x-v.y)*.2)v+=cos(v.yx*i+vec2(0,i)+t)/i+.7;o=tanh(exp(p.y*vec4(1,-1,-2,0))*exp(-4.*l.x)/o);
        */

        const WIDTH: u16 = 200;
        const HEIGHT: u16 = 200;

        let colors = (0..WIDTH).flat_map(|y| {
            (0..HEIGHT).map(move |x| {
                let mut p = Vector2::new(x as f32, y as f32);
                let r = Vector2::new(WIDTH as f32, HEIGHT as f32);
                p = (p * 2.0 - r) / r.y;
                let mut l: f32 = 0.0;
                let pdotp = p.dot(p);
                l += (0.7f32 - pdotp).abs();
                let mut v = p * (1.0 - l) / 0.2f32;
                // Initialize 'o' as a Vector4, not a Vector2
                let mut o = Vector4::new(0.0, 0.0, 0.0, 0.0);

                for i in 1..=8 {
                    let fi = i as f32;

                    // 1. sin(v.xyyx) -> Expand v to a Vector4 and take the sin
                    let v_xyyx = Vector4::new(v.x, v.y, v.y, v.x);

                    // o += (sin(v.xyyx)+1.) * abs(v.x-v.y) * .2
                    o = o
                        + (v_xyyx.sin() + Vector4::new(1.0, 1.0, 1.0, 1.0))
                            * (v.x - v.y).abs()
                            * 0.2f32;

                    // 2. v += cos(v.yx*i+vec2(0,i)+t)/i+.7;
                    let v_yx = Vector2::new(v.y, v.x); // Swizzle to yx
                    let t = cnt as f32; // Assuming 't' is your time/frame counter

                    let inner_calc = v_yx * fi + Vector2::new(0.0, fi) + t;
                    v = v + (inner_calc.cos() / fi) + Vector2::new(0.7, 0.7);
                }

                // 3. o=tanh(exp(p.y*vec4(1,-1,-2,0))*exp(-4.*l.x)/o);
                let exp_vec = (Vector4::new(1.0, -1.0, -2.0, 0.0) * p.y).exp();
                let exp_scalar = libm::expf(-4.0 * l);

                // Assuming Vector4 has a method to do element-wise division and tanh()
                o = (exp_vec * exp_scalar / o).tanh();
                Rgb565::new((o.x * 31.0) as u8, (o.y * 63.0) as u8, (o.z * 31.0) as u8)
            })
        });
        display.fill_colors(0, 0, WIDTH, HEIGHT, colors).unwrap();

        cnt += 1;
    }
}

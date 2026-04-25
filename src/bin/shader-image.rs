#![no_std]
#![no_main]

extern crate alloc;

use core::mem::MaybeUninit;
use embedded_graphics::image::{Image, ImageRaw, ImageRawBE, ImageRawLE};
use embedded_graphics::mono_font::{MonoTextStyle, MonoTextStyleBuilder};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Alignment, Text};
use esp_backtrace as _;
use esp_println::println;

use esp_hal::{
    delay::Delay,
    gpio::{Io, Level, Output, OutputConfig},
    spi::master::{Config as SpiConfig, Spi},
    spi::Mode,
    time::Rate,
};

esp_bootloader_esp_idf::esp_app_desc!();

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

impl core::ops::Add<f32> for Vector4 {
    type Output = Self;

    fn add(self, scalar: f32) -> Self {
        Self {
            x: self.x + scalar,
            y: self.y + scalar,
            z: self.z + scalar,
            w: self.w + scalar,
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

#[esp_hal::main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);

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

    let mut cnt = 0.0f32;

    const WIDTH: usize = 60;
    const HEIGHT: usize = 60;
    let mut image_data = [0u8; WIDTH * HEIGHT * 2];

    loop {
        let r_y = HEIGHT as f32;
        let r_vec = Vector2::new(WIDTH as f32, HEIGHT as f32);

        for y in 0..HEIGHT {
            println!("{y}");
            for x in 0..WIDTH {
                let mut p = Vector2::new(x as f32, y as f32);
                // Normalized coordinates: -1.0 to 1.0 range (roughly)
                p = (p * 2.0 - r_vec) / r_y;

                let pdotp = p.dot(p);
                let l = (0.7f32 - pdotp).abs();
                let mut v = p * (1.0 - l) * 5.0; // Optimized division (1/0.2 = 5)

                let mut o = Vector4::new(0.0, 0.0, 0.0, 0.0);

                for i in 1..=8 {
                    let fi = i as f32;
                    let v_xyyx = Vector4::new(v.x, v.y, v.y, v.x);

                    // o += (sin(v.xyyx)+1.) * abs(v.x-v.y) * .2
                    o = o + (v_xyyx.sin() + 1.0) * (v.x - v.y).abs() * 0.2;

                    let v_yx = Vector2::new(v.y, v.x);
                    let inner_calc = v_yx * fi + Vector2::new(0.0, fi) + cnt;
                    // Pre-calculate 1.0/fi to avoid division if possible
                    v = v + (inner_calc.cos() / fi) + 0.7;
                }

                let exp_vec = (Vector4::new(1.0, -1.0, -2.0, 0.0) * p.y).exp();
                let exp_scalar = libm::expf(-4.0 * l);

                o = (exp_vec * exp_scalar / o).tanh();

                // Map 0.0..1.0 to Rgb565 bit depths
                // Use clamping to ensure values stay in range
                let r = (o.x.clamp(0.0, 1.0) * 31.0) as u16;
                let g = (o.y.clamp(0.0, 1.0) * 63.0) as u16;
                let b = (o.z.clamp(0.0, 1.0) * 31.0) as u16;

                let pixel_color = Rgb565::new(r as u8, g as u8, b as u8);
                let idx = (x + y * WIDTH) * 2;

                // Direct byte manipulation is faster than to_be_bytes calls
                let raw = pixel_color.to_be_bytes();
                image_data[idx] = raw[0];
                image_data[idx + 1] = raw[1];
            }
        }

        let raw_image = ImageRawBE::<Rgb565>::new(&image_data, WIDTH as u32);
        Image::new(&raw_image, Point::new(10, 10))
            .draw(&mut display)
            .unwrap();

        cnt += 1.0; // Increment time
    }
}

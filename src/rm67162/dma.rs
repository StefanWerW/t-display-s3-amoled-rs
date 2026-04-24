//! On-board RM67162 AMOLED screen driver

use core::iter;

use embedded_graphics::{
    pixelcolor::{raw::ToBytes, Rgb565},
    prelude::{DrawTarget, OriginDimensions, Size},
    primitives::Rectangle,
    Pixel,
};
use embedded_hal_1::{delay::DelayNs, digital::OutputPin};

use hal::{
    spi::master::{SpiDmaBus, Command, Address, DataMode},
    Blocking,
};

use crate::rm67162::Orientation;

pub const SCREEN_SIZE: Size = Size::new(240, 536);

const BUFFER_PIXELS: usize = 16368 / 2;
const BUFFER_SIZE: usize = BUFFER_PIXELS * 2;
static mut DMA_BUFFER: [u8; BUFFER_SIZE] = [0u8; BUFFER_SIZE];
static mut DMA_DESCRIPTORS: [hal::dma::DmaDescriptor; 4] = [hal::dma::DmaDescriptor::EMPTY; 4];

pub type SpiType<'d> =
    SpiDmaBus<'d, Blocking>;

pub struct RM67162Dma<'a, CS, DC> {
    spi: Option<SpiType<'a>>,
    cs: CS,
    dc: DC,
    orientation: Orientation,
}

impl<CS, DC> RM67162Dma<'_, CS, DC>
where
    CS: OutputPin,
    DC: OutputPin,
{
    pub fn new<'a>(
        spi: SpiType<'a>,
        cs: CS,
        dc: DC,
    ) -> RM67162Dma<'a, CS, DC> {
        RM67162Dma {
            spi: Some(spi),
            cs,
            dc,
            orientation: Orientation::Portrait,
        }
    }

    pub fn set_orientation(&mut self, orientation: Orientation) -> Result<(), ()> {
        self.orientation = orientation;

        self.send_cmd(0x36, &[self.orientation.to_madctr()])
    }

    pub fn reset(&self, rst: &mut impl OutputPin, delay: &mut impl DelayNs) -> Result<(), ()> {
        rst.set_low().unwrap();
        delay.delay_ms(300);

        rst.set_high().unwrap();
        delay.delay_ms(200);
        Ok(())
    }

    fn send_cmd(&mut self, cmd: u32, data: &[u8]) -> Result<(), ()> {
        self.cs.set_low().unwrap();
        self.dc.set_low().unwrap();

        let mut spi = self.spi.take().unwrap();
        
        // Write command
        let cmd_buf = [cmd as u8];
        spi.half_duplex_write(
            DataMode::Single,
            Command::None,
            Address::None,
            0,
            &cmd_buf,
        ).unwrap();

        self.dc.set_high().unwrap();

        // Write data
        if !data.is_empty() {
            spi.half_duplex_write(
                DataMode::Single,
                Command::None,
                Address::None,
                0,
                data,
            ).unwrap();
        }

        self.spi.replace(spi);
        self.cs.set_high().unwrap();
        Ok(())
    }

    pub fn init(&mut self, delay: &mut impl embedded_hal_1::delay::DelayNs) -> Result<(), ()> {
        for _ in 0..3 {
            self.send_cmd(0xFE, &[0x00])?;
            self.send_cmd(0x11, &[])?; // sleep out
            delay.delay_ms(120);

            self.send_cmd(0x3A, &[0x75])?; // 16bit mode

            self.send_cmd(0x51, &[0x00])?; // write brightness

            self.send_cmd(0x29, &[])?; // display on
            delay.delay_ms(120);

            self.send_cmd(0x51, &[0xD0])?; // write brightness
        }

        self.set_orientation(self.orientation)?;
        Ok(())
    }

    pub fn set_address(&mut self, x1: u16, y1: u16, x2: u16, y2: u16) -> Result<(), ()> {
        self.send_cmd(
            0x2a,
            &[
                (x1 >> 8) as u8,
                (x1 & 0xFF) as u8,
                (x2 >> 8) as u8,
                (x2 & 0xFF) as u8,
            ],
        )?;
        self.send_cmd(
            0x2b,
            &[
                (y1 >> 8) as u8,
                (y1 & 0xFF) as u8,
                (y2 >> 8) as u8,
                (y2 & 0xFF) as u8,
            ],
        )?;
        self.send_cmd(0x2c, &[])?;
        Ok(())
    }

    fn draw_point(&mut self, x: u16, y: u16, color: Rgb565) -> Result<(), ()> {
        self.set_address(x, y, x, y)?;

        self.cs.set_low().unwrap();
        self.dc.set_low().unwrap();

        let mut spi = self.spi.take().unwrap();
        
        // Write command
        let cmd_buf = [0x2C];
        spi.half_duplex_write(
            DataMode::Single,
            Command::None,
            Address::None,
            0,
            &cmd_buf,
        ).unwrap();

        self.dc.set_high().unwrap();

        let raw = color.to_be_bytes();
        spi.half_duplex_write(
            DataMode::Single,
            Command::None,
            Address::None,
            0,
            &raw,
        ).unwrap();

        self.spi.replace(spi);
        self.cs.set_high().unwrap();
        Ok(())
    }

    #[inline]
    fn dma_send_colors(&mut self, txbuf: &[u8], first_send: bool) -> Result<(), ()> {
        if first_send {
            self.dc.set_low().unwrap();
            let mut spi = self.spi.take().unwrap();
            let cmd_buf = [0x2C];
            spi.half_duplex_write(
                DataMode::Single,
                Command::None,
                Address::None,
                0,
                &cmd_buf,
            ).unwrap();
            self.spi.replace(spi);
            self.dc.set_high().unwrap();
        }

        let mut spi = self.spi.take().unwrap();
        spi.half_duplex_write(DataMode::Single, Command::None, Address::None, 0, txbuf)
                .unwrap();

        self.spi.replace(spi);
        Ok(())
    }

    // fill a rectangle with raw color buffer, the buffer must be in 16bit rgb565 format
    pub unsafe fn fill_raw_colors(
        &mut self,
        x: u16,
        y: u16,
        w: u16,
        h: u16,
        raw_colors: &[u8],
    ) -> Result<(), ()> {
        self.set_address(x, y, x + w - 1, y + h - 1)?;

        self.cs.set_low().unwrap();
        self.dma_send_colors(raw_colors, true)?;
        self.cs.set_high().unwrap();
        Ok(())
    }

    /// Use a framebuffer to fill the screen.
    /* Framebuffer::<
        Rgb565,
        _,
        BigEndian,
        536,
        240,
        { embedded_graphics::framebuffer::buffer_size::<Rgb565>(536, 240) },
    > */
    pub unsafe fn fill_with_framebuffer(&mut self, raw_framebuffer: &[u8]) -> Result<(), ()> {
        self.set_address(
            0,
            0,
            self.size().width as u16 - 1,
            self.size().height as u16 - 1,
        )?;

        let mut first_send = true;
        self.cs.set_low().unwrap();

        for chunk in raw_framebuffer.chunks(BUFFER_SIZE) {
            self.dma_send_colors(chunk, first_send)?;
            first_send = false;
        }

        self.cs.set_high().unwrap();
        Ok(())
    }

    pub fn fill_colors(
        &mut self,
        x: u16,
        y: u16,
        w: u16,
        h: u16,
        colors: impl Iterator<Item = Rgb565>,
    ) -> Result<(), ()> {
        self.set_address(x, y, x + w - 1, y + h - 1)?;

        let mut first_send = true;
        self.cs.set_low().unwrap();

        let mut i = 0;

        for color in colors.into_iter().take(w as usize * h as usize) {
            if i == BUFFER_PIXELS {
                self.dma_send_colors(unsafe { &DMA_BUFFER }, first_send)?;
                first_send = false;
                i = 0;
            }
            unsafe {
                DMA_BUFFER[2 * i..2 * i + 2].copy_from_slice(&color.to_be_bytes()[..]);
            }
            i += 1;
        }
        if i > 0 {
            self.dma_send_colors(unsafe { &DMA_BUFFER[..2 * i] }, first_send)?;
        }
        self.cs.set_high().unwrap();
        Ok(())
    }

    fn fill_color(&mut self, x: u16, y: u16, w: u16, h: u16, color: Rgb565) -> Result<(), ()> {
        self.fill_colors(x, y, w, h, iter::repeat(color))?;
        Ok(())
    }
}

impl<CS, DC> OriginDimensions for RM67162Dma<'_, CS, DC>
where
    CS: OutputPin,
    DC: OutputPin,
{
    fn size(&self) -> Size {
        if matches!(
            self.orientation,
            Orientation::Landscape | Orientation::LandscapeFlipped
        ) {
            Size::new(536, 240)
        } else {
            Size::new(240, 536)
        }
    }
}

impl<CS, DC> DrawTarget for RM67162Dma<'_, CS, DC>
where
    CS: OutputPin,
    DC: OutputPin,
{
    type Color = Rgb565;

    type Error = ();

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        for Pixel(pt, color) in pixels {
            if pt.x < 0 || pt.y < 0 {
                continue;
            }

            self.draw_point(pt.x as u16, pt.y as u16, color)?;
        }
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        self.fill_color(
            area.top_left.x as u16,
            area.top_left.y as u16,
            area.size.width as u16,
            area.size.height as u16,
            color,
        )?;
        Ok(())
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        self.fill_colors(
            area.top_left.x as u16,
            area.top_left.y as u16,
            area.size.width as u16,
            area.size.height as u16,
            colors.into_iter(),
        )?;
        Ok(())
    }
}


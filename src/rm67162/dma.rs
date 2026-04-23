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
    spi::master::{SpiDma, Command, Address, DataMode},
    Blocking,
};

use crate::rm67162::Orientation;

pub const SCREEN_SIZE: Size = Size::new(240, 536);

const BUFFER_PIXELS: usize = 16368 / 2;
const BUFFER_SIZE: usize = BUFFER_PIXELS * 2;
static mut DMA_BUFFER: [u8; BUFFER_SIZE] = [0u8; BUFFER_SIZE];
static mut DMA_DESCRIPTORS: [hal::dma::DmaDescriptor; 4] = [hal::dma::DmaDescriptor::EMPTY; 4];

pub type SpiType<'d> =
    SpiDma<'d, Blocking>;

pub struct RM67162Dma<'a, CS> {
    spi: Option<SpiType<'a>>,
    cs: CS,
    orientation: Orientation,
}

impl<CS> RM67162Dma<'_, CS>
where
    CS: OutputPin,
{
    pub fn new<'a>(
        spi: SpiType<'a>,
        cs: CS,
    ) -> RM67162Dma<'a, CS> {
        RM67162Dma {
            spi: Some(spi),
            cs,
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
        let txbuf = StaticReadBuffer::new(data.as_ptr(), data.len());
        self.cs.set_low().unwrap();

        let mut spi = self.spi.take().unwrap();
        let tx = spi
            .half_duplex_write(
                DataMode::Single,
                Command::_8Bit(0x02, DataMode::Single),
                Address::_24Bit(cmd << 8, DataMode::Single),
                0,
                data.len(),
                txbuf,
            )
            .unwrap();
        let (spi_back, _) = tx.wait();
        spi = spi_back;
        self.spi.replace(spi);

        self.cs.set_high().unwrap();
        Ok(())
    }

    // rm67162_qspi_init
    pub fn init(&mut self, delay: &mut impl embedded_hal_1::delay::DelayNs) -> Result<(), ()> {
        for _ in 0..3 {
            self.send_cmd(0x11, &[])?; // sleep out
            delay.delay_ms(120);

            self.send_cmd(0x3A, &[0x55])?; // 16bit mode

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

        let raw = color.to_be_bytes();
        let txbuf = StaticReadBuffer::new(raw.as_ptr(), 2);

        self.cs.set_low().unwrap();

        let mut spi = self.spi.take().unwrap();
        let tx = spi
            .half_duplex_write(
                DataMode::Quad,
                Command::_8Bit(0x32, DataMode::Single),
                Address::_24Bit(0x2C << 8, DataMode::Single),
                0,
                2,
                txbuf,
            )
            .unwrap();
        let (spi_back, _) = tx.wait();
        spi = spi_back;
        self.spi.replace(spi);

        self.cs.set_high().unwrap();
        Ok(())
    }

    #[inline]
    fn dma_send_colors(&mut self, txbuf: StaticReadBuffer, first_send: bool) -> Result<(), ()> {
        let mut spi = self.spi.take().unwrap();
        let len = txbuf.len;

        let tx = if first_send {
            spi.half_duplex_write(
                DataMode::Quad,
                Command::_8Bit(0x32, DataMode::Single),
                Address::_24Bit(0x2C << 8, DataMode::Single),
                0,
                len,
                txbuf,
            )
            .unwrap()
        } else {
            spi.half_duplex_write(DataMode::Quad, Command::None, Address::None, 0, len, txbuf)
                .unwrap()
        };
        let (spi_back, _) = tx.wait();
        spi = spi_back;
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
        let txbuf = StaticReadBuffer::new(raw_colors.as_ptr(), raw_colors.len());
        self.dma_send_colors(txbuf, true)?;
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
            let txbuf = StaticReadBuffer::new(chunk.as_ptr(), chunk.len());
            self.dma_send_colors(txbuf, first_send)?;
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
                let txbuf = StaticReadBuffer::new(unsafe { DMA_BUFFER.as_ptr() }, BUFFER_SIZE);

                self.dma_send_colors(txbuf, first_send)?;
                first_send = false;
                i = 0;
            }
            unsafe {
                DMA_BUFFER[2 * i..2 * i + 2].copy_from_slice(&color.to_be_bytes()[..]);
            }
            i += 1;
        }
        if i > 0 {
            let txbuf = StaticReadBuffer::new(unsafe { DMA_BUFFER.as_ptr() }, 2 * i);
            self.dma_send_colors(txbuf, first_send)?;
        }

        self.cs.set_high().unwrap();
        Ok(())
    }

    fn fill_color(&mut self, x: u16, y: u16, w: u16, h: u16, color: Rgb565) -> Result<(), ()> {
        self.fill_colors(x, y, w, h, iter::repeat(color))?;
        Ok(())
    }
}

impl<CS> OriginDimensions for RM67162Dma<'_, CS>
where
    CS: OutputPin,
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

impl<CS> DrawTarget for RM67162Dma<'_, CS>
where
    CS: OutputPin,
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

#[derive(Copy, Clone, Debug)]
pub struct StaticReadBuffer {
    buffer: *const u8,
    len: usize,
}

impl StaticReadBuffer {
    pub fn new(buffer: *const u8, len: usize) -> StaticReadBuffer {
        StaticReadBuffer { buffer, len }
    }
}

unsafe impl hal::dma::DmaTxBuffer for StaticReadBuffer {
    type View = Self;
    type Final = Self;

    fn prepare(&mut self) -> hal::dma::Preparation {
        let len = self.len;
        let mut ptr = self.buffer as *mut u8;
        
        let mut remaining = len;
        let mut i = 0;
        unsafe {
            if len == 0 {
                DMA_DESCRIPTORS[0] = hal::dma::DmaDescriptor::EMPTY;
                DMA_DESCRIPTORS[0].set_length(0);
                DMA_DESCRIPTORS[0].set_size(0);
                DMA_DESCRIPTORS[0].set_owner(hal::dma::Owner::Dma);
                DMA_DESCRIPTORS[0].set_suc_eof(true);
                DMA_DESCRIPTORS[0].buffer = core::ptr::null_mut();
            } else {
                while remaining > 0 {
                    let chunk_len = core::cmp::min(remaining, 4092);
                    DMA_DESCRIPTORS[i] = hal::dma::DmaDescriptor::EMPTY;
                    DMA_DESCRIPTORS[i].set_length(chunk_len);
                    DMA_DESCRIPTORS[i].set_size(chunk_len);
                    DMA_DESCRIPTORS[i].buffer = ptr;
                    DMA_DESCRIPTORS[i].set_owner(hal::dma::Owner::Dma);
                    
                    remaining -= chunk_len;
                    ptr = ptr.add(chunk_len);
                    
                    if remaining > 0 {
                        DMA_DESCRIPTORS[i].next = DMA_DESCRIPTORS.as_mut_ptr().add(i + 1);
                        DMA_DESCRIPTORS[i].set_suc_eof(false);
                    } else {
                        DMA_DESCRIPTORS[i].next = core::ptr::null_mut();
                        DMA_DESCRIPTORS[i].set_suc_eof(true);
                    }
                    i += 1;
                }
            }
        }
        
        hal::dma::Preparation {
            start: unsafe { DMA_DESCRIPTORS.as_mut_ptr() },
            direction: hal::dma::TransferDirection::Out,
            accesses_psram: false,
            burst_transfer: hal::dma::BurstConfig::default(),
            check_owner: Some(true),
            auto_write_back: false,
        }
    }

    fn into_view(self) -> Self::View {
        self
    }

    fn from_view(view: Self::View) -> Self::Final {
        view
    }
}

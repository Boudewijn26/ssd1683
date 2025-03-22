//! Driver for interacting with SSD1683 display driver
use core::fmt::Debug;

use embedded_graphics::pixelcolor::BinaryColor;
use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
    spi::SpiBus,
};
#[cfg(feature="log")]
use log::debug;

use crate::{cmd, color, flag, HEIGHT, WIDTH};
use crate::{color::Color, interface::DisplayInterface};

fn split_u16(value: u16) -> (u8, u8) {
    let high_byte = (value >> 8) as u8; // Extract the upper 8 bits
    let low_byte = (value & 0xFF) as u8; // Extract the lower 8 bits
    (high_byte, low_byte)
}

// Go here if you fuck up
// Check: are you waiting for the activation?

/// A configured display with a hardware interface.
pub struct Ssd1683<SPI, BUSY, DC, RST> {
    interface: DisplayInterface<SPI, BUSY, DC, RST>,
}

impl<SPI, BUSY, DC, RST> Ssd1683<SPI, BUSY, DC, RST>
where
    SPI: SpiBus<u8>,
    BUSY: InputPin,
    DC: OutputPin,
    DC::Error: Debug,
    RST: OutputPin,
    RST::Error: Debug,
{
    /// Create and initialize the display driver
    pub fn new<DELAY: DelayNs>(
        spi: &mut SPI,
        busy: BUSY,
        dc: DC,
        rst: RST,
        delay: &mut DELAY,
    ) -> Result<Self, SPI::Error>
    where
        Self: Sized,
    {
        let interface = DisplayInterface::new(busy, dc, rst);
        let mut ssd1683 = Ssd1683 { interface };
        ssd1683.init(spi, delay)?;
        Ok(ssd1683)
    }

    /// Initialise the controller
    pub fn init<DELAY: DelayNs>(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
    ) -> Result<(), SPI::Error> {
        self.interface.reset(delay);
        // self.interface.wait_until_idle();
        self.interface.cmd(spi, cmd::SW_RESET)?;
        self.interface.wait_until_idle();
        delay.delay_ms(10);

        self.interface
            .cmd_with_data(spi, cmd::UPDATE_DISPLAY_CTRL1, &[0x40, 0x00])?; // 0x40 = A6 = 1
        let (high, low) = split_u16(HEIGHT - 1);

        self.interface.cmd_with_data(
            spi,
            cmd::BORDER_WAVEFORM_CONTROL,
            &[flag::BORDER_WAVEFORM_FOLLOW_LUT | flag::BORDER_WAVEFORM_LUT1],
        )?;
        // self.interface
        //     .cmd_with_data(spi, cmd::SET_TEMPERATURE_REGISTER, &[0x6E])?; // 1.5 s

        // TODO: weird, 0x99 doesn't load temp, B9 does
        // self.interface
        //     .cmd_with_data(spi, cmd::UPDATE_DISPLAY_CTRL2, &[0xB9])?; // load temp value

        // self.interface
        //     .cmd_with_data(spi, cmd::DRIVER_CONTROL, &[low, high, 0x00])?;

        // self.interface.cmd(spi, cmd::MASTER_ACTIVATE)?;

        // self.interface
        //     .cmd_with_data(spi, cmd::DATA_ENTRY_MODE, &[0x11])?;
        self.interface
            .cmd_with_data(spi, cmd::DATA_ENTRY_MODE, &[flag::DATA_ENTRY_INCRY_INCRX])?;

        self.use_full_frame(spi)?;

        self.interface
            .cmd_with_data(spi, cmd::TEMP_CONTROL, &[flag::INTERNAL_TEMP_SENSOR])?;

        self.interface.wait_until_idle();
        Ok(())
    }

    /// Update the whole BW buffer on the display driver
    pub fn update_bw_frame(&mut self, spi: &mut SPI, buffer: &[u8]) -> Result<(), SPI::Error> {
        self.use_full_frame(spi)?;
        self.interface
            .cmd_with_data(spi, cmd::WRITE_BW_DATA, &buffer)
    }

    /// Update the whole Red buffer on the display driver
    pub fn update_red_frame(&mut self, spi: &mut SPI, buffer: &[u8]) -> Result<(), SPI::Error> {
        self.use_full_frame(spi)?;
        self.interface
            .cmd_with_data(spi, cmd::WRITE_RED_DATA, &buffer)
    }

    /// Start an update of the whole display
    pub fn display_frame(&mut self, spi: &mut SPI) -> Result<(), SPI::Error> {
        // self.interface
        //     .cmd_with_data(spi, cmd::UPDATE_DISPLAY_CTRL1, &[0x40, 0x00])?;
        self.interface
            .cmd_with_data(spi, cmd::UPDATE_DISPLAY_CTRL2, &[flag::DISPLAY_MODE_1])?;
        self.interface.cmd(spi, cmd::MASTER_ACTIVATE)?;

        self.interface.wait_until_idle();

        Ok(())
    }

    /// Make the whole black and white frame on the display driver white
    pub fn clear_bw_frame(&mut self, spi: &mut SPI, color: Color) -> Result<(), SPI::Error> {
        self.use_full_frame(spi)?;

        let color = color.get_byte_value();
        let reps = (u32::from(WIDTH) * u32::from(HEIGHT)) / 8;
        #[cfg(feature="log")]
        debug!("reps: {}, col: {}", reps, color);

        self.interface.cmd(spi, cmd::WRITE_BW_DATA)?;
        self.interface.data_x_times(spi, color, reps)?;
        Ok(())
    }

    /// Make the whole red frame on the display driver white
    pub fn clear_red_frame(&mut self, spi: &mut SPI) -> Result<(), SPI::Error> {
        self.use_full_frame(spi)?;

        // TODO: allow non-white background color
        let color = color::Color::White.inverse().get_byte_value();

        self.interface.cmd(spi, cmd::WRITE_RED_DATA)?;
        self.interface
            .data_x_times(spi, color, (u32::from(WIDTH) * u32::from(HEIGHT)) / 8)?;
        Ok(())
    }

    fn use_full_frame(&mut self, spi: &mut SPI) -> Result<(), SPI::Error> {
        // choose full frame/ram
        self.set_ram_area(spi, 0, 0, u32::from(WIDTH) - 1, u32::from(HEIGHT) - 1)?;

        // start from the beginning
        self.set_ram_counter(spi, 0, 0)
    }

    fn set_ram_area(
        &mut self,
        spi: &mut SPI,
        start_x: u32,
        start_y: u32,
        end_x: u32,
        end_y: u32,
    ) -> Result<(), SPI::Error> {
        assert!(start_x < end_x);
        assert!(start_y < end_y);

        let x_pos = [(start_x >> 3) as u8, (end_x >> 3) as u8];
        #[cfg(feature="log")]
        debug!("x pos _ {:?}", x_pos);

        self.interface.cmd_with_data(
            spi,
            cmd::SET_RAMXPOS,
            &x_pos,
        )?;

        let y_pos = [start_y as u8, (start_y >> 8) as u8, end_y as u8, (end_y >> 8) as u8];
        #[cfg(feature="log")]
        debug!("y pos _ {:?}", y_pos);

        self.interface.cmd_with_data(
            spi,
            cmd::SET_RAMYPOS,
            &y_pos,
        )?;
        Ok(())
    }

    fn set_ram_counter(&mut self, spi: &mut SPI, x: u32, y: u32) -> Result<(), SPI::Error> {
        // x is positioned in bytes, so the last 3 bits which show the position inside a byte in the ram
        // aren't relevant
        let x_ram_counter = [(x >> 3) as u8];
        #[cfg(feature="log")]
        debug!("x ram counter _ {:?}", x_ram_counter);
        self.interface
            .cmd_with_data(spi, cmd::SET_RAMX_COUNTER, &x_ram_counter)?;

        let y_ram_counter = [y as u8, (y >> 8) as u8];
        #[cfg(feature="log")]
        debug!("y ram counter _ {:?}", y_ram_counter);
        // 2 Databytes: A[7:0] & 0..A[8]
        self.interface
            .cmd_with_data(spi, cmd::SET_RAMY_COUNTER, &y_ram_counter)?;
        Ok(())
    }

    // pub fn wake_up<DELAY: DelayMs<u8>>(
    //     &mut self,
    //     spi: &mut SPI,
    //     delay: &mut DELAY,
    // ) -> Result<(), SPI::Error> {
    //     todo!()
    // }
}

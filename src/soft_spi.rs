//! jlkfsd
use embedded_hal::{digital::OutputPin, spi::SpiBus};

/// SA
pub struct SoftSpi<CS, MOSI, SCK>
where
    CS: OutputPin,
    MOSI: OutputPin,
    SCK: OutputPin,
{
    cs: CS,
    mosi: MOSI,
    sck: SCK,
}

impl<CS, MOSI, SCK> SoftSpi<CS, MOSI, SCK>
where
    CS: OutputPin,
    MOSI: OutputPin,
    SCK: OutputPin,
{
    /// fsdoi
    pub fn new(cs: CS, mosi: MOSI, sck: SCK) -> Self {
        SoftSpi { cs, mosi, sck }
    }
}
impl<CS, MOSI, SCK> embedded_hal::spi::ErrorType for SoftSpi<CS, MOSI, SCK>
where
    CS: OutputPin,
    MOSI: OutputPin,
    SCK: OutputPin,
{
    type Error = core::convert::Infallible;
}

impl<CS, MOSI, SCK> SpiBus<u8> for SoftSpi<CS, MOSI, SCK>
where
    CS: OutputPin,
    MOSI: OutputPin,
    SCK: OutputPin,
{
    fn read(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        panic!("not implemented")
    }

    fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        self.cs.set_low().ok();
        for &word in words {
            let mut dat = word;
            for _ in 0..8 {
                self.sck.set_low().unwrap();
                if dat & 0x80 != 0 {
                    self.mosi.set_high().unwrap();
                } else {
                    self.mosi.set_low().unwrap();
                }
                self.sck.set_high().unwrap();
                dat <<= 1;
            }
        }
        self.cs.set_high().ok();
        Ok(())
    }

    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error> {
        panic!("not implemented")
    }

    fn transfer_in_place(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        panic!("not implemented")
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

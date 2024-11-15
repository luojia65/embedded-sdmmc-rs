use super::crc7;

/// Abstract SD card transportation interface.
pub trait Transport {
    /// Transport error type.
    type Error;

    /// Write command and argument to the card.
    fn write_card_command(&mut self, command: u8, arg: u32) -> Result<(), Self::Error>;

    /// Read one-byte card response from the card.
    fn read_card_response_u8(&mut self) -> Result<u8, Self::Error>;

    /// Read four-byte card response from the card.
    fn read_card_response_u32(&mut self) -> Result<u32, Self::Error>;

    /// Read sixteen-byte card response from the card.
    fn read_card_response_u128(&mut self) -> Result<u128, Self::Error>;

    /// Write data to the card.
    fn write_data(&mut self, buf: &[u8]) -> Result<(), Self::Error>;

    /// Read data from the card.
    fn read_data(&mut self, buf: &mut [u8]) -> Result<(), Self::Error>;

    /// Try to flush the card.
    fn flush_card(&mut self) -> Result<(), Self::Error>;

    /// Gets if the card is busy.
    fn is_busy(&mut self) -> Result<bool, Self::Error>;
}

/// SPI as an abstract SD card transportation.
pub struct SpiTransport<SPI> {
    spi: SPI,
}

impl<SPI> SpiTransport<SPI>
where
    SPI: embedded_hal::spi::SpiDevice<u8>,
{
    /// Create a new SD/MMC transpotation interface using a raw SPI interface.
    #[inline]
    pub fn new(spi: SPI) -> Self {
        Self { spi }
    }
    /// Get a temporary borrow on the underlying SPI device.
    #[inline]
    pub fn spi<T, F>(&mut self, func: F) -> T
    where
        F: FnOnce(&mut SPI) -> T,
    {
        func(&mut self.spi)
    }
    /// Release the underlying SPI and free the interface.
    #[inline]
    pub fn free(self) -> SPI {
        self.spi
    }
}

impl<SPI> SpiTransport<SPI>
where
    SPI: embedded_hal::spi::SpiDevice<u8>,
{
    /// Send one byte and receive one byte from the card.
    #[inline]
    fn transfer_byte(&mut self, byte: u8) -> Result<u8, SPI::Error> {
        let mut read_buf = [0u8; 1];
        self.spi.transfer(&mut read_buf, &[byte])?;
        Ok(read_buf[0])
    }
}

impl<SPI> Transport for SpiTransport<SPI>
where
    SPI: embedded_hal::spi::SpiDevice<u8>,
{
    type Error = SPI::Error;

    #[inline]
    fn write_card_command(&mut self, command: u8, arg: u32) -> Result<(), Self::Error> {
        let mut buf = [
            0x40 | command,
            (arg >> 24) as u8,
            (arg >> 16) as u8,
            (arg >> 8) as u8,
            arg as u8,
            0,
        ];
        buf[5] = crc7(&buf[0..5]);
        self.spi.write(&buf)
    }

    #[inline]
    fn read_card_response_u8(&mut self) -> Result<u8, Self::Error> {
        let mut read_buf = [0];
        let write_buf = [0xFF];
        self.spi.transfer(&mut read_buf, &write_buf)?;
        Ok(read_buf[0])
    }

    #[inline]
    fn read_card_response_u32(&mut self) -> Result<u32, Self::Error> {
        let mut read_buf = [0; 4];
        let write_buf = [0xFF; 4];
        self.spi.transfer(&mut read_buf, &write_buf)?;
        Ok(u32::from_be_bytes(read_buf))
    }

    #[inline]
    fn read_card_response_u128(&mut self) -> Result<u128, Self::Error> {
        let mut read_buf = [0; 16];
        let write_buf = [0xFF; 16];
        self.spi.transfer(&mut read_buf, &write_buf)?;
        Ok(u128::from_be_bytes(read_buf))
    }

    #[inline]
    fn write_data(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        self.spi.write(&buf)
    }

    #[inline]
    fn read_data(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        buf.fill(0xFF);
        self.spi.transfer_in_place(buf)
    }

    #[inline]
    fn flush_card(&mut self) -> Result<(), Self::Error> {
        // Try flushing the card as done here:
        // https://github.com/greiman/SdFat/blob/master/src/SdCard/SdSpiCard.cpp#L170,
        // https://github.com/rust-embedded-community/embedded-sdmmc-rs/pull/65#issuecomment-1270709448
        for _ in 0..0xFF {
            self.transfer_byte(0xFF)?;
        }
        Ok(())
    }

    #[inline]
    fn is_busy(&mut self) -> Result<bool, Self::Error> {
        match self.transfer_byte(0xFF)? {
            0xFF => Ok(false),
            _ => Ok(true),
        }
    }
}

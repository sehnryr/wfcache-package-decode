use std::io::{BufRead, Cursor, Read, Seek, Write};
use std::result::Result::Ok;

use anyhow::Result;
use encoding::all::ISO_8859_1;
use encoding::{DecoderTrap, Encoding};
use zstd::{stream::raw::DParameter, Decoder};
use zstd_safe::FrameFormat;

mod package;
mod path;

fn main() -> Result<()> {
    let path = "/home/youn/Downloads/Packages.bin/Packages.1.bin";
    let file = std::fs::File::open(path)?;
    let package = package::Package::from(file);

    let mut decompressed_zstd_file = std::fs::File::create("decompressed_zstd.bin")?;

    let mut package_decoder = PackageDecoder::new(
        Cursor::new(package.zstd_raw_data),
        package.zstd_dictionary.clone(),
    )?;

    let mut buffer1_offset: usize = 0;
    let mut buffer2_offset: usize = 0;
    for _ in 0..package.bottom_paths_number {
        // Get the flag that indicates if the data is present in the ZSTD buffer.
        let is_data_present = package.data1_raw.read_bit(buffer1_offset) == 1;
        buffer1_offset += 1;

        // If the data is not present, then continue to the next iteration.
        if !is_data_present {
            continue;
        }

        // Get the flag that indicates if the data is compressed.
        let is_compressed = package.data1_raw.read_bit(buffer1_offset) == 1;
        buffer1_offset += 1;

        // Get the size of the compressed frame.
        let mut compressed_size_buffer = &package.data2_raw[4 + buffer2_offset..];
        let compressed_size = compressed_size_buffer.read_var_usize()?;
        buffer2_offset = package.data2_raw.len() - 4 - compressed_size_buffer.len();

        // Decompress or read the frame.
        let data = match is_compressed {
            true => package_decoder.decompress_zstd_frame()?,
            false => package_decoder.read_raw(compressed_size)?,
        };

        // Decode the data as ISO-8859-1.
        let value = ISO_8859_1.decode(&data, DecoderTrap::Strict).unwrap();

        // Write the data to the file.
        decompressed_zstd_file.write_all(value.as_bytes())?;
    }

    Ok(())
}

/// Trait for reading a single bit from a buffer.
///
/// The buffer is expected to be a slice of bytes.
trait ReadBit {
    /// Reads a single bit from the buffer at the specified offset and returns it.
    fn read_bit(&self, offset: usize) -> u8;
}

impl ReadBit for [u8] {
    fn read_bit(&self, offset: usize) -> u8 {
        let index = offset >> 3;
        self[index] >> (offset & 7) & 1
    }
}

/// Trait for reading a variable-length integer from a reader.
///
/// The integer is encoded as a variable-length integer, where each byte
/// contains 7 bits of the integer, and the MSB indicates whether the
/// next byte is part of the integer.
trait ReadVarInteger {
    /// Reads a variable-length unsigned 32-bit integer from the reader.
    fn read_var_u32(&mut self) -> Result<u32>;

    /// Reads a variable-length unsigned size from the reader.
    fn read_var_usize(&mut self) -> Result<usize> {
        self.read_var_u32().map(|value| value as usize)
    }
}

impl<R: BufRead> ReadVarInteger for R {
    fn read_var_u32(&mut self) -> Result<u32> {
        let mut value: u32 = 0;
        let mut shift: u32 = 0;
        loop {
            let mut buf = [0u8; 1];
            self.read_exact(&mut buf)?;
            let byte = buf[0] as u32;
            value |= (byte & 0b0111_1111) << shift;
            shift += 7;
            if byte & 0b1000_0000 == 0 {
                break;
            }
        }
        Ok(value)
    }
}

struct PackageDecoder<'a, R: BufRead + Seek> {
    decoder: Decoder<'a, R>,
}

impl<R: BufRead + Seek> PackageDecoder<'_, R> {
    pub fn new(reader: R, dictionnary: Vec<u8>) -> Result<Self> {
        let mut decoder = Decoder::with_dictionary(reader, &dictionnary)?;
        decoder.set_parameter(DParameter::Format(FrameFormat::Magicless))?;
        Ok(Self { decoder })
    }

    /// Decompresses a single ZSTD frame.
    pub fn decompress_zstd_frame(&mut self) -> Result<Vec<u8>> {
        let decompressed_size = self.decoder.get_mut().read_var_usize()?;

        let mut decompressed: Vec<u8> = Vec::new();
        decompressed.resize(decompressed_size, 0);
        self.decoder.read_exact(decompressed.as_mut())?;

        Ok(decompressed)
    }

    /// Reads the raw data from the ZSTD buffer.
    pub fn read_raw(&mut self, size: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; size];
        self.decoder.get_mut().read_exact(buf.as_mut())?;

        // Remove the last two bytes from the frame.
        // They are always 0x0A 0x00 and signify the end of the frame.
        if size > 2 && buf[size - 2] == 0x0A && buf[size - 1] == 0x00 {
            buf.truncate(size - 2);
        }

        Ok(buf)
    }
}

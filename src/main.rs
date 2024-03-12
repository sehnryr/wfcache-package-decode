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
        let is_data_present = read_bit(&package.data1_raw, buffer1_offset) == 1;
        buffer1_offset += 1;

        // If the data is not present, then continue to the next iteration.
        if !is_data_present {
            continue;
        }

        // Get the flag that indicates if the data is compressed.
        let is_compressed = read_bit(&package.data1_raw, buffer1_offset) == 1;
        buffer1_offset += 1;

        // Get the size of the compressed frame.
        let (compressed_size, size_size) = read_varuint(&package.data2_raw[4..], buffer2_offset);
        let compressed_size = compressed_size as usize;
        buffer2_offset += size_size;

        // Decompress or read the frame.
        let data = match is_compressed {
            true => decompress_zstd_frame(&mut package_decoder)?,
            false => read_plain(package_decoder.get_mut(), compressed_size)?,
        };

        // Decode the data as ISO-8859-1.
        let value = ISO_8859_1.decode(&data, DecoderTrap::Strict).unwrap();

        // Write the data to the file.
        decompressed_zstd_file.write_all(value.as_bytes())?;
    }

    Ok(())
}

fn read_bit(buffer: &[u8], offset: usize) -> u8 {
    let index = offset >> 3;
    buffer[index] >> (offset & 7) & 1
}

fn read_varuint(buffer: &[u8], offset: usize) -> (u32, usize) {
    let mut value: u32 = 0;
    let mut shift: u32 = 0;
    let mut size: usize = 0;
    loop {
        let byte = buffer[offset + size] as u32;
        value |= (byte & 0b0111_1111) << shift;
        shift += 7;
        size += 1;
        if byte & 0b1000_0000 == 0 {
            break;
        }
    }
    (value, size)
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

    pub fn get_mut(&mut self) -> &mut R {
        self.decoder.get_mut()
    }

    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        self.decoder.read_exact(buf)?;
        Ok(())
    }
}

/// Reads the size of the decompressed frame from the compressed frame.
///
/// The size is encoded as a variable-length integer, where each byte
/// contains 7 bits of the size, and the MSB indicates whether the
/// next byte is part of the size.
fn read_zstd_frame_size<R: BufRead>(mut reader: R) -> Result<usize> {
    let mut value: usize = 0;
    let mut shift: usize = 0;
    loop {
        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf)?;
        let byte = buf[0] as usize;
        value |= (byte & 0b0111_1111) << shift;
        shift += 7;
        if byte & 0b1000_0000 == 0 {
            break;
        }
    }
    Ok(value)
}

/// Decompresses a single ZSTD frame.
fn decompress_zstd_frame<R: BufRead + Seek>(decoder: &mut PackageDecoder<R>) -> Result<Vec<u8>> {
    let decompressed_size = read_zstd_frame_size(decoder.get_mut())?;

    let mut decompressed: Vec<u8> = Vec::new();
    decompressed.resize(decompressed_size, 0);
    decoder.read_exact(decompressed.as_mut())?;

    Ok(decompressed)
}

fn read_plain<R: BufRead>(mut reader: R, size: usize) -> Result<Vec<u8>> {
    let mut buf = vec![0u8; size];
    reader.read_exact(buf.as_mut())?;

    // Remove the last two bytes.
    // They are always 0x0A 0x00 and signify the end of the frame.
    buf.truncate(size - 2);
    Ok(buf)
}

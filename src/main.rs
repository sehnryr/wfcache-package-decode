use std::io::{BufRead, Cursor, Read, Seek, Write};
use std::result::Result::Ok;

use anyhow::{Error, Result};
use encoding::all::ISO_8859_1;
use encoding::{DecoderTrap, Encoding};
use replace_with::replace_with_or_abort;
use zstd::{stream::raw::DParameter, Decoder};
use zstd_safe::FrameFormat;

mod package;
mod path;

fn main() -> Result<()> {
    // let path = "../wfcache-api/Extracted/Packages.bin.H(1).raw";
    let path = "/home/youn/Downloads/Packages.bin_OG";
    let file = std::fs::File::open(path).unwrap();
    let package = package::Package::from(file);

    // let mut compressed_cursor = Cursor::new(package.zstd_raw_data.clone());

    let mut decompressed_zstd_file = std::fs::File::create("decompressed_zstd.bin").unwrap();

    let mut package_decoder = PackageDecoder::new(
        Cursor::new(package.zstd_raw_data),
        package.zstd_dictionary.clone(),
    )?;

    loop {
        // Break if the cursor is at the end of the compressed data.
        if package_decoder.get_ref().position() >= package_decoder.get_ref().get_ref().len() as u64
        {
            break;
        }

        let decompressed_frame = decompress_frame(&mut package_decoder)?;
        let value = ISO_8859_1
            .decode(&decompressed_frame, DecoderTrap::Strict)
            .unwrap();
        decompressed_zstd_file.write_all(value.as_bytes())?;
    }

    Ok(())
}

struct PackageDecoder<'a, R: BufRead + Seek> {
    dictionnary: Vec<u8>,
    decoder: Decoder<'a, R>,
}

impl<R: BufRead + Seek> PackageDecoder<'_, R> {
    pub fn new(reader: R, dictionnary: Vec<u8>) -> Result<Self> {
        let mut decoder = Decoder::with_dictionary(reader, &dictionnary)?;
        decoder.set_parameter(DParameter::Format(FrameFormat::Magicless))?;
        Ok(Self {
            dictionnary,
            decoder,
        })
    }

    pub fn rebuild(&mut self) -> Result<()> {
        replace_with_or_abort(&mut self.decoder, |decoder| {
            let reader = decoder.finish();
            Decoder::with_dictionary(reader, &self.dictionnary).unwrap()
        });
        self.decoder
            .set_parameter(DParameter::Format(FrameFormat::Magicless))?;
        Ok(())
    }

    pub fn get_mut(&mut self) -> &mut R {
        self.decoder.get_mut()
    }

    pub fn get_ref(&self) -> &R {
        self.decoder.get_ref()
    }

    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        self.decoder.read_exact(buf)?;
        Ok(())
    }
}

/// Checks the frame header of a ZSTD frame.
fn check_frame_header<R: BufRead + Seek>(mut reader: R, decompressed_size: usize) -> Result<()> {
    let mut buf = [0u8; 1];
    reader.read_exact(&mut buf)?;
    let frame_header_descriptor = buf[0];

    let single_segment_flag = frame_header_descriptor >> 5 & 0b1;
    let dictionary_id_flag = frame_header_descriptor & 0b11;

    // If the dictionary ID flag is 0, then it's should be a correct frame.
    if dictionary_id_flag == 0 {
        reader.seek(std::io::SeekFrom::Current(-1))?; // Rewind the reader.
        return Ok(());
    }

    // Check the window descriptor if the single segment flag is 0.
    if single_segment_flag == 0 {
        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf)?;
        let window_descriptor = buf[0];

        let exponent = window_descriptor >> 3 & 0b11111;
        let mantissa = window_descriptor & 0b111;

        let window_log: usize = 10 + exponent as usize;
        let window_base: usize = 1 << window_log;
        let window_add: usize = (window_base >> 3) * mantissa as usize;
        let window_size: usize = window_base + window_add;

        if (window_size) < decompressed_size {
            return Ok(());
        } else {
            reader.seek(std::io::SeekFrom::Current(-2))?; // Rewind the reader.
            return Err(Error::msg("Invalid frame header"));
        }
    }

    let mut buf = Vec::<u8>::new();
    buf.resize(dictionary_id_flag as usize, 0);
    reader.read_exact(&mut buf)?;

    let dictionary_id = buf.iter().fold(0, |acc, &byte| acc << 8 | byte as u32);

    // If the dictionary ID is not 0, then it's not a correct frame.
    if dictionary_id != 0 {
        reader.seek(std::io::SeekFrom::Current(-1))?; // Rewind the reader.
        return Err(Error::msg("Invalid frame header"));
    }

    Ok(())
}

/// Reads the size of the decompressed frame from the compressed frame.
///
/// The size is encoded as a variable-length integer, where each byte
/// contains 7 bits of the size, and the MSB indicates whether the
/// next byte is part of the size.
fn decompressed_zstd_frame_size<R: BufRead>(mut reader: R) -> Result<usize> {
    let mut size_bits = Vec::<u8>::new();
    let mut size = 0;

    // Read bytes until the MSB is 0.
    loop {
        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf)?;
        let byte = buf[0];

        // If the MSB is 0, then this is the last byte of the size.
        if byte & 0b1 << 7 == 0 {
            size_bits.push(byte);
            break;
        }

        // Otherwise, the MSB is 1, so we need to read the next byte.
        size_bits.push(byte & 0b0111_1111);
    }

    // Combine the bytes into a usize.
    for (i, byte) in size_bits.iter().enumerate() {
        size |= (*byte as usize) << (i * 7);
    }
    Ok(size)
}

/// Decompresses a single ZSTD frame.
fn decompress_zstd_frame<R: BufRead + Seek>(decoder: &mut PackageDecoder<R>) -> Result<Vec<u8>> {
    let initial_position = decoder.get_mut().seek(std::io::SeekFrom::Current(0))?;
    let decompressed_size = decompressed_zstd_frame_size(decoder.get_mut())?;

    // Check the frame header.
    match check_frame_header(decoder.get_mut(), decompressed_size) {
        Ok(_) => {}
        Err(_) => {
            decoder
                .get_mut()
                .seek(std::io::SeekFrom::Start(initial_position))?; // Rewind the reader.
            return Err(Error::msg("Invalid frame header"));
        }
    }

    let mut decompressed: Vec<u8> = Vec::new();
    decompressed.resize(decompressed_size, 0);
    match decoder.read_exact(decompressed.as_mut()) {
        Ok(_) => {}
        Err(_) => {
            decoder.rebuild()?;
            decoder
                .get_mut()
                .seek(std::io::SeekFrom::Start(initial_position))?; // Rewind the reader.
            return Err(Error::msg("Invalid frame"));
        }
    }

    Ok(decompressed)
}

fn decompress_frame<R: BufRead + Seek>(decoder: &mut PackageDecoder<R>) -> Result<Vec<u8>> {
    // Try to decompress the frame as a ZSTD frame.
    match decompress_zstd_frame(decoder) {
        Ok(decompressed) => return Ok(decompressed),
        Err(_) => {}
    }

    // Try to decompress the frame as a plain frame.
    match read_plain(decoder.get_mut()) {
        Ok(decompressed) => return Ok(decompressed),
        Err(_) => {}
    }

    Err(Error::msg("Invalid frame"))
}

fn read_plain<R: BufRead>(mut reader: R) -> Result<Vec<u8>> {
    let mut decompressed = Vec::<u8>::new();
    let mut end_sequence = false;

    // Read bytes until finding the sequence 0x0A 0x00.
    loop {
        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf)?;
        let byte = buf[0];

        if byte == 0x0A {
            end_sequence = true;
        } else if end_sequence && byte == 0x00 {
            break;
        } else if end_sequence {
            end_sequence = false;
        }

        decompressed.push(byte);
    }

    Ok(decompressed)
}

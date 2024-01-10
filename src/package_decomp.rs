// Functional decompression of the 4th data block of a package file.

use std::io::{Read, Write};

use zstd::{Decoder, stream::raw::DParameter};
use zstd_safe::FrameFormat;

mod package;
mod path;

fn main() {
    let path = "../wfcache-api/Extracted/Packages.bin.H(1).raw";
    let file = std::fs::File::open(path).unwrap();
    let package = package::Package::from(file);

    let dictionary = package.data3[0..0x100000].to_vec();
    let compressed = package.data3[0x100000..].to_vec();

    // The byte is the decompressed length of the following ZSTD frame.
    let compressed = compressed[1..].to_vec();

    let mut decoder = Decoder::with_dictionary(compressed.as_slice(), &dictionary).unwrap();
    let _ = decoder.set_parameter(DParameter::Format(FrameFormat::Magicless));
    let mut decoder = decoder.single_frame();

    let mut decompressed: Vec<u8> = Vec::new();
    let n = decoder.read_to_end(decompressed.as_mut());

    if n.is_ok() {
        println!("Decompressed {} bytes", n.unwrap());
        let mut file = std::fs::File::create("decompressed.bin").unwrap();
        file.write_all(&decompressed).unwrap();
    } else {
        println!("Error: {:?}", n.err());
    }
}

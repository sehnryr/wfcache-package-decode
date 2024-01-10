// Functional decompression of languages.bin files.

use std::io::Read;

use zstd::{stream::raw::DParameter, Decoder};
use zstd_safe::FrameFormat;

mod language;
mod package;
mod path;

fn main() {
    let path = "../wfcache-api/Extracted/Languages.bin.H(1).raw";
    let file = std::fs::File::open(path).unwrap();
    let language = language::Language::from(file);

    let dictionary = language.zstd_dictionary;

    let section = &language.sections[6];

    let compressed = section.data.clone();

    for localization_header in &section.localization_headers {
        let localization_position = localization_header.localization_position as usize;
        let localization_length = localization_header.localization_length as usize;

        println!(
            "Compression type: {:#X}",
            localization_header.compression_type
        );

        let localization_bytes = match localization_header.compression_type {
            0x02 => decompress(
                &compressed[localization_position + 1..localization_position + localization_length],
                &dictionary,
            ),
            _ => section.data[localization_position..localization_position + localization_length]
                .to_vec(),
        };
        let localization = String::from_utf8(localization_bytes).unwrap();
        let unique_name = String::from_utf8(localization_header.unique_name.clone()).unwrap();

        println!("{}: {}", unique_name, localization);
    }
}

fn decompress(data: &[u8], dictionary: &[u8]) -> Vec<u8> {
    let mut decoder = Decoder::with_dictionary(data, dictionary).unwrap();
    let _ = decoder.set_parameter(DParameter::Format(FrameFormat::Magicless));

    let mut decompressed: Vec<u8> = Vec::new();
    let n = decoder.read_to_end(decompressed.as_mut());

    if n.is_ok() {
        decompressed
    } else {
        Vec::new()
    }
}

use std::{fs::File, io::Read};

use crate::path::Path;

pub struct Language {
    pub hash: [u8; 16],
    pub unknown1: u32,
    pub unknown2: u32,
    pub unknown3: u32,
    pub lang_number: u32,
    pub langs: Vec<Path>,
    pub zstd_dictionary_length: u32,
    pub zstd_dictionary: Vec<u8>,
    pub section_number: u32,
    pub sections: Vec<Section>,
}

pub struct LocalizationHeader {
    pub unique_name_length: u32,
    pub unique_name: Vec<u8>,
    pub localization_position: u32,
    pub localization_length: u16,
    pub unknown1: u8,
    pub compression_type: u8,
}

pub struct Section {
    pub title_length : u32,
    pub title : Vec<u8>,
    pub data_length : u32,
    pub data : Vec<u8>,
    pub localization_header_number : u32,
    pub localization_headers : Vec<LocalizationHeader>,
    pub _length: u32,
}

impl From<File> for Language {
    fn from(mut file: File) -> Self {
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        Self::from(bytes.as_slice())
    }
}

impl From<&[u8]> for Language {
    fn from(bytes: &[u8]) -> Self {
        let hash = bytes[0..16].try_into().unwrap();
        let unknown1 = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
        let unknown2 = u32::from_le_bytes(bytes[20..24].try_into().unwrap());
        let unknown3 = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
        let lang_number = u32::from_le_bytes(bytes[28..32].try_into().unwrap());
        let mut langs = Vec::new();
        let mut offset = 32;
        for _ in 0..lang_number {
            let lang = Path::from(&bytes[offset..]);
            offset += 4 + lang.length as usize;
            langs.push(lang);
        }
        let zstd_dictionary_length = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let zstd_dictionary = bytes[offset..offset + zstd_dictionary_length as usize].to_vec();
        offset += zstd_dictionary_length as usize;
        let section_number = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let mut sections = Vec::new();
        for _ in 0..section_number {
            let section = Section::from(&bytes[offset..]);
            offset += section._length as usize;
            sections.push(section);
        }
        Self {
            hash,
            unknown1,
            unknown2,
            unknown3,
            lang_number,
            langs,
            zstd_dictionary_length,
            zstd_dictionary,
            section_number,
            sections,
        }
    }
}

impl From<&[u8]> for LocalizationHeader {
    fn from(bytes: &[u8]) -> Self {
        let unique_name_length = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let unique_name = bytes[4..4 + unique_name_length as usize].to_vec();
        let localization_position = u32::from_le_bytes(bytes[unique_name_length as usize + 4..unique_name_length as usize + 8].try_into().unwrap());
        let localization_length = u16::from_le_bytes(bytes[unique_name_length as usize + 8..unique_name_length as usize + 10].try_into().unwrap());
        let unknown1 = bytes[unique_name_length as usize + 10];
        let compression_type = bytes[unique_name_length as usize + 11];
        Self {
            unique_name_length,
            unique_name,
            localization_position,
            localization_length,
            unknown1,
            compression_type,
        }
    }
}

impl From<&[u8]> for Section {
    fn from(bytes: &[u8]) -> Self {
        let title_length = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let title = bytes[4..4 + title_length as usize].to_vec();
        let data_length = u32::from_le_bytes(bytes[4 + title_length as usize..8 + title_length as usize].try_into().unwrap());
        let data = bytes[8 + title_length as usize..8 + title_length as usize + data_length as usize].to_vec();
        let mut offset = title_length as usize + data_length as usize + 8;
        let localization_header_number = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let mut localization_headers = Vec::new();
        for _ in 0..localization_header_number {
            let localization_header = LocalizationHeader::from(&bytes[offset..]);
            offset += localization_header.unique_name_length as usize + 12;
            localization_headers.push(localization_header);
        }
        let length = offset as u32;
        Self {
            title_length,
            title,
            data_length,
            data,
            localization_header_number,
            localization_headers,
            _length: length,
        }
    }
}

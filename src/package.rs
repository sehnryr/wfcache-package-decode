use std::{fs::File, io::Read};

use crate::path::Path;

pub struct Package {
    pub hash: [u8; 16],
    pub file_type: u32,
    pub unknown2: u32,
    pub unknown3: u32,
    pub unknown4: u32,
    pub paths: Vec<Path>,
    pub data1_raw: Vec<u8>,
    pub data2_raw: Vec<u8>,
    pub zstd_dictionary: Vec<u8>,
    pub zstd_raw_data: Vec<u8>,
    pub bottom_paths_raw: Vec<u8>,
}

impl From<File> for Package {
    fn from(mut file: File) -> Self {
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        Self::from(bytes.as_slice())
    }
}

impl From<&[u8]> for Package {
    fn from(bytes: &[u8]) -> Self {
        let hash = bytes[0..16].try_into().unwrap();
        let file_type = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
        let unknown2 = u32::from_le_bytes(bytes[20..24].try_into().unwrap());
        let unknown3 = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
        let unknown4 = u32::from_le_bytes(bytes[28..32].try_into().unwrap());

        let path_number = u32::from_le_bytes(bytes[32..36].try_into().unwrap());
        let mut paths = Vec::new();
        let mut offset = 36;
        for _ in 0..path_number {
            let path = Path::from(&bytes[offset..]);
            offset += 6 + path.length as usize;
            paths.push(path);
        }

        // Skip 4 empty bytes.
        offset += 4;

        let data1_length = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let data1_raw = bytes[offset..offset + data1_length as usize].to_vec();
        offset += data1_length as usize;

        let data2_length = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let data2_raw = bytes[offset..offset + data2_length as usize].to_vec();
        offset += data2_length as usize;

        let zstd_block_length = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let zstd_raw = bytes[offset..offset + zstd_block_length as usize].to_vec();
        let zstd_dictionary = zstd_raw[0..0x100000].to_vec();
        let zstd_raw_data = zstd_raw[0x100000..].to_vec();
        offset += zstd_block_length as usize;

        let bottom_path_number = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let bottom_paths_raw = bytes[offset..offset + bottom_path_number as usize].to_vec();

        Self {
            hash,
            file_type,
            unknown2,
            unknown3,
            unknown4,
            paths,
            data1_raw,
            data2_raw,
            zstd_dictionary,
            zstd_raw_data,
            bottom_paths_raw,
        }
    }
}

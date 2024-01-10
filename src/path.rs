pub struct Path {
    pub length: u32,
    pub name: String,
    pub unknown1: u8,
    pub unknown2: u8,
}

impl From<&[u8]> for Path {
    fn from(bytes: &[u8]) -> Self {
        let length = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let name = String::from_utf8(bytes[4..4 + length as usize].to_vec()).unwrap();
        let unknown1 = bytes[4 + length as usize];
        let unknown2 = bytes[5 + length as usize];
        Self {
            length,
            name,
            unknown1,
            unknown2,
        }
    }
}

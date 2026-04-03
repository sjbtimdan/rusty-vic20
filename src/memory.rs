pub struct Memory {
    pub bytes: [u8; 65536],
}

impl Default for Memory {
    fn default() -> Self {
        Self { bytes: [0; 65536] }
    }
}
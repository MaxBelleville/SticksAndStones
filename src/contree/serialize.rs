pub struct BitstreamWriter {
    pub bits: Vec<u8>,
    pub bit_offset: usize,
}

impl BitstreamWriter {
    pub fn new() -> Self {
        BitstreamWriter {
            bits: Vec::new(),
            bit_offset: 0,
        }
    }

    pub fn write_bits(&mut self, mut value: u32, mut count: usize) {
        while count > 0 {
            let byte_index = self.bit_offset / 8;
            let bit_in_byte = self.bit_offset % 8;

            if byte_index >= self.bits.len() {
                self.bits.push(0);
            }

            let space_in_byte = 8 - bit_in_byte;
            let bits_to_write = count.min(space_in_byte);
            let mask = (1u32 << bits_to_write) - 1;
            let bits = (value & mask) as u8;

            self.bits[byte_index] |= bits << bit_in_byte;

            self.bit_offset += bits_to_write;
            value >>= bits_to_write;
            count -= bits_to_write;
        }
    }

    pub fn write_2bits(&mut self, value: u8) {
        self.write_bits(value as u32, 2);
    }

    pub fn write_byte(&mut self, value: u8) {
        self.align_to_byte();
        self.bits.push(value);
        self.bit_offset += 8;
    }

    pub fn write_u16(&mut self, value: u16) {
        self.align_to_byte();
        let bytes = value.to_le_bytes();
        self.bits.push(bytes[0]);
        self.bits.push(bytes[1]);
        self.bit_offset += 16;
    }

    pub fn reserve_u16(&mut self) -> usize {
        self.align_to_byte();
        let pos = self.bits.len();
        self.bits.push(0);
        self.bits.push(0);
        self.bit_offset += 16;
        pos
    }

    pub fn write_u16_at(&mut self, pos: usize, value: u16) {
        let bytes = value.to_le_bytes();
        self.bits[pos] = bytes[0];
        self.bits[pos + 1] = bytes[1];
    }

    pub fn total_bits(&self) -> usize {
        self.bit_offset
    }

    pub fn align_to_byte(&mut self) {
        let remainder = self.bit_offset % 8;
        if remainder != 0 {
            self.bit_offset += 8 - remainder;
            let byte_index = self.bit_offset / 8;
            if byte_index > self.bits.len() {
                self.bits.push(0);
            }
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.bits
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.bits
    }
}

pub struct BitstreamReader<'a> {
    pub bits: &'a [u8],
    pub bit_offset: usize,
}

impl<'a> BitstreamReader<'a> {
    pub fn new(bits: &'a [u8]) -> Self {
        BitstreamReader {
            bits,
            bit_offset: 0,
        }
    }

    pub fn get_bit_offset(&self) -> usize {
        self.bit_offset
    }

    pub fn set_bit_offset(&mut self, offset: usize) {
        self.bit_offset = offset;
    }

    pub fn read_2bits(&mut self) -> Option<u8> {
        self.read_bits(2)
    }

    pub fn read_byte(&mut self) -> Option<u8> {
        self.read_bits(8)
    }

    pub fn read_u16(&mut self) -> Option<u16> {
        let lo = self.read_bits(8)? as u16;
        let hi = self.read_bits(8)? as u16;
        Some(lo | (hi << 8))
    }

    fn read_bits(&mut self, count: usize) -> Option<u8> {
        let mut val = 0u8;
        for i in 0..count {
            let byte_index = self.bit_offset / 8;
            let bit_in_byte = self.bit_offset % 8;

            if byte_index >= self.bits.len() {
                return None;
            }

            let bit = (self.bits[byte_index] >> bit_in_byte) & 1;
            val |= bit << i;
            self.bit_offset += 1;
        }
        Some(val)
    }
}

pub struct BitReader<'a> {
    buffer: &'a [u32],
    offset: usize,
}

impl<'a> BitReader<'a> {
    #[inline]
    pub fn new(buffer: &'a [u32], offset: usize) -> Self {
        Self { buffer, offset }
    }

    #[inline]
    pub unsafe fn read_bits_unchecked(&mut self, num_bits: u32) -> u32 {
        let bit_idx = self.offset & 31;
        let idx = self.offset >> 5;

        self.offset += num_bits as usize;

        let mut value = *self.buffer.get_unchecked(idx) >> bit_idx;
        if bit_idx as u32 + num_bits > 32 {
            value |= self.buffer.get_unchecked(idx + 1) << (32 - bit_idx);
        }

        value & ((1 << num_bits) - 1)
    }
}

#[cfg(test)]
mod tests {
    use std::slice;

    use super::*;

    #[test]
    fn read_unchecked() {
        use std::io::Cursor;

        use bitstream_io::{BitWrite, BitWriter, LittleEndian};

        let bytes = {
            let mut writer = BitWriter::<_, LittleEndian>::new(Cursor::new(Vec::new()));

            for i in 2..20 {
                writer.write(i, (1 << i) - 2).unwrap();
            }

            writer.byte_align().unwrap();

            let mut bytes = writer.into_writer().into_inner();
            while (bytes.len() & 3) != 0 {
                bytes.push(0);
            }

            bytes
        };

        let buffer = unsafe { slice::from_raw_parts(bytes.as_ptr().cast(), bytes.len() >> 2) };

        let mut reader = BitReader::new(&buffer, 0);
        for i in 2..20 {
            unsafe {
                assert_eq!(reader.read_bits_unchecked(i), (1 << i) - 2);
            }
        }
    }
}

use std::fmt;

use crate::bitops::{leading_ones_mask, ones_mask, single_one_mask};

#[derive(Debug, Clone, PartialEq)]
pub struct Bitmap {
    pub(crate) capacity: usize,
    length: usize,
    pub(crate) data: Vec<u64>,
}

impl Bitmap {
    pub fn new(size: usize, capacity: usize) -> Self {
        let data_size = (size + 63) / 64;
        let data_capacity = (capacity + 63) / 64;

        let mut data = Vec::with_capacity(data_capacity);
        data.resize(data_size, 0);

        Bitmap {
            capacity,
            length: data_size * 64,
            data,
        }
    }

    pub fn set(&mut self, bit: usize) -> Result<(), &'static str> {
        if bit >= self.capacity {
            return Err("Invalid index. Must be in range [0, capacity - 1]");
        }

        if bit >= self.length {
            self.resize(bit + 1);
        }

        let idx = bit / 64;
        let offset = bit % 64;
        let mask = single_one_mask(offset as u32);

        self.data[idx] |= mask;
        Ok(())
    }

    pub fn unset(&mut self, bit: usize) -> Result<(), &'static str> {
        if bit >= self.capacity {
            return Err("Invalid index. Must be in range [0, capacity - 1]");
        }

        if bit >= self.length {
            self.resize(bit + 1);
        }

        let idx = bit / 64;
        let offset = bit % 64;
        let mask = ones_mask(offset as u32, (64 - offset - 1) as u32);

        self.data[idx] &= mask;
        Ok(())
    }

    pub fn get(&mut self, bit: usize) -> Result<u8, &'static str> {
        if bit >= self.capacity {
            return Err("Invalid index. Must be in range [0, capacity - 1]");
        }

        if bit >= self.length {
            self.resize(bit + 1);
        }

        let idx = bit / 64;
        let offset = bit % 64;
        let mask = single_one_mask(offset as u32);

        let val = (self.data[idx] & mask) >> (64 - offset - 1);
        Ok(val as u8)
    }

    pub fn select(&mut self, val: u8, nth: usize) -> Result<usize, &'static str> {
        if val != 0 && val != 1 {
            return Err("Val must be one of 0, 1");
        }

        if nth == 0 || nth > self.length {
            return Err("Nth must be in [1, length]");
        }

        let check_ones = val == 1;
        let mut count = 0;
        let mut idx = 0;

        while idx < self.length {
            let mut additional_count = self.data[idx / 64].count_ones() as usize;
            if !check_ones {
                additional_count = 64 - additional_count;
            }

            if count + additional_count >= nth {
                break;
            }

            count += additional_count;
            idx += 64;
        }

        if idx >= self.length {
            return Err("Bitmap only contained nth bits of value val");
        }

        while count < nth {
            let bit_val = self.get(idx)?;
            if (bit_val == 1 && check_ones) || (bit_val == 0 && !check_ones) {
                count += 1;
            }
            idx += 1;
        }

        Ok(idx - 1)
    }

    pub fn rank(&mut self, val: u8, idx: usize) -> Result<usize, &'static str> {
        if idx >= self.length {
            return Err("Index must be in range [0, length - 1]");
        }

        if val != 0 && val != 1 {
            return Err("Val must be one of 0, 1");
        }

        let check_ones = val == 1;
        let mut count = 0;

        for i in (0..=idx).step_by(64) {
            let full_block = i + 63 < idx;

            let ones_count = if full_block {
                self.data[i / 64].count_ones() as usize
            } else {
                let mask = leading_ones_mask((idx - i + 1) as u32);
                (self.data[i / 64] & mask).count_ones() as usize
            };

            if check_ones {
                count += ones_count;
            } else if full_block {
                count += 64 - ones_count;
            } else {
                count += idx - i + 1 - ones_count;
            }
        }

        Ok(count)
    }

    fn resize(&mut self, bit: usize) {
        if bit < self.length {
            return;
        }

        let bit = bit.min(self.capacity);
        let new_length = (bit + 63) / 64;
        let _additional_uints = new_length - self.data.len();

        self.data.resize(new_length, 0);
        self.length = new_length * 64;
    }
}

impl fmt::Display for Bitmap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let max_offset_length = (self.length as f64).log10().ceil() as usize;
        let pattern = format!("%0{max_offset_length}d |");

        for i in (0..self.data.len() * 64).step_by(64) {
            write!(f, "{}{}", pattern, i)?;

            let bits_string = format!("{:064b}", self.data[i / 64]);

            for j in (0..64).step_by(8) {
                let b = &bits_string[j..j + 8];
                write!(f, " {}", b)?;
            }

            writeln!(f)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitmap_set() {
        let mut bitmap = Bitmap::new(64, 64);

        let keys = [0, 7, 16, 63];
        for key in keys {
            bitmap.set(key).unwrap();
        }

        assert_eq!(
            bitmap.data[0], 0x8100800000000001,
            "bitmap.data[0] == 0x8100800000000001"
        );
    }

    #[test]
    fn test_set_get_and_unset() {
        let mut bitmap = Bitmap::new(256, 256);

        for key in 0..256 {
            let val = bitmap.get(key).unwrap();
            assert_eq!(val, 0, "bitmap.get({}) == 0", key);
            bitmap.set(key).unwrap();
            assert_eq!(bitmap.get(key).unwrap(), 1, "bitmap.get({}) == 1", key);
        }

        for key in 0..256 {
            bitmap.unset(key).unwrap();
            assert_eq!(bitmap.get(key).unwrap(), 0, "bitmap.get({}) == 0", key);
        }
    }
}

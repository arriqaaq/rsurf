pub fn first_bits(n: u32, b: u64) -> u64 {
    b & leading_ones_mask(n)
}

pub fn last_bits(n: u32, b: u64) -> u64 {
    b & trailing_ones_mask(n)
}

pub fn leading_ones_mask(n: u32) -> u64 {
    let n = n.clamp(0, 64);
    if n == 0 {
        0
    } else {
        u64::MAX << (64 - n)
    }
}

pub fn trailing_ones_mask(n: u32) -> u64 {
    let n = n.clamp(0, 64);
    if n == 0 {
        0
    } else {
        u64::MAX >> (64 - n)
    }
}

pub fn ones_mask(leading: u32, trailing: u32) -> u64 {
    let leading = leading.clamp(0, 64);
    let trailing = trailing.clamp(0, 64);

    let left = leading_ones_mask(leading);
    let right = trailing_ones_mask(trailing);

    left | right
}

pub fn single_one_mask(idx: u32) -> u64 {
    let idx = idx.clamp(0, 63);
    0x8000000000000000 >> idx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_first_bits() {
        let b: u64 = 0b0001101111001100000111111010100110101111111011110101000010100001;
        assert_eq!(
            first_bits(1, b),
            0b0000000000000000000000000000000000000000000000000000000000000000
        );
        assert_eq!(
            first_bits(3, b),
            0b0000000000000000000000000000000000000000000000000000000000000000
        );
        assert_eq!(
            first_bits(17, b),
            0b0001101111001100000000000000000000000000000000000000000000000000
        );
        assert_eq!(
            first_bits(62, b),
            0b0001101111001100000111111010100110101111111011110101000010100000
        );
        assert_eq!(
            first_bits(64, b),
            0b0001101111001100000111111010100110101111111011110101000010100001
        );
    }

    #[test]
    fn test_last_bits() {
        let b: u64 = 0b0001101111001100000111111010100110101111111011110101000010100001;
        assert_eq!(
            last_bits(1, b),
            0b0000000000000000000000000000000000000000000000000000000000000001
        );
        assert_eq!(
            last_bits(3, b),
            0b0000000000000000000000000000000000000000000000000000000000000001
        );
        assert_eq!(
            last_bits(17, b),
            0b0000000000000000000000000000000000000000000000010101000010100001
        );
        assert_eq!(
            last_bits(62, b),
            0b0001101111001100000111111010100110101111111011110101000010100001
        );
        assert_eq!(
            last_bits(64, b),
            0b0001101111001100000111111010100110101111111011110101000010100001
        );
    }

    #[test]
    fn test_leading_ones_mask() {
        assert_eq!(leading_ones_mask(0), 0x0000000000000000);
        assert_eq!(leading_ones_mask(1), 0x8000000000000000);
        assert_eq!(leading_ones_mask(2), 0xC000000000000000);
        assert_eq!(leading_ones_mask(3), 0xE000000000000000);
        assert_eq!(leading_ones_mask(4), 0xF000000000000000);
        assert_eq!(leading_ones_mask(8), 0xFF00000000000000);
        assert_eq!(leading_ones_mask(32), 0xFFFFFFFF00000000);
        assert_eq!(leading_ones_mask(62), 0xFFFFFFFFFFFFFFFC);
        assert_eq!(leading_ones_mask(64), 0xFFFFFFFFFFFFFFFF);
        assert_eq!(leading_ones_mask(70), 0xFFFFFFFFFFFFFFFF);
    }

    #[test]
    fn test_trailing_ones_mask() {
        assert_eq!(trailing_ones_mask(0), 0x0000000000000000);
        assert_eq!(trailing_ones_mask(1), 0x0000000000000001);
        assert_eq!(trailing_ones_mask(2), 0x0000000000000003);
        assert_eq!(trailing_ones_mask(3), 0x0000000000000007);
        assert_eq!(trailing_ones_mask(4), 0x000000000000000F);
        assert_eq!(trailing_ones_mask(8), 0x00000000000000FF);
        assert_eq!(trailing_ones_mask(32), 0x00000000FFFFFFFF);
        assert_eq!(trailing_ones_mask(62), 0x3FFFFFFFFFFFFFFF);
        assert_eq!(trailing_ones_mask(64), 0xFFFFFFFFFFFFFFFF);
        assert_eq!(trailing_ones_mask(70), 0xFFFFFFFFFFFFFFFF);
    }
}

// Key represents a single key that can be stored in a LOUDS-encoded FST tree.
pub(crate) type Key = Vec<u8>;

// KeyOps defines methods that can be used on Key types.
pub(crate) trait KeyOps {
    // Less compares two keys lexicographically.
    //
    // It compares pairs of corresponding bytes of the two keys at the same index.
    // If the two bytes differ, the key with the lesser byte is considered lesser.
    //
    // If all pairs of corresponding bytes are equal, the key with the lesser
    // length is considered lesser.
    //
    // If the two keys are equal, neither is considered lesser than the other.
    fn less(&self, other: &[u8]) -> bool;
}

impl KeyOps for Vec<u8> {
    fn less(&self, other: &[u8]) -> bool {
        let min_length = std::cmp::min(self.len(), other.len());

        for i in 0..min_length {
            match self[i].cmp(&other[i]) {
                std::cmp::Ordering::Less => return true,
                std::cmp::Ordering::Greater => return false,
                std::cmp::Ordering::Equal => continue,
            }
        }

        // Shared prefix is equal, so self is lesser if it is shorter
        self.len() < other.len()
    }
}

// Truncate truncates the list of keys such that they are still uniquely
// identifiable.
//
// The inputs must be sorted and may not contain any duplicates. Then, each key
// is truncated to as short a prefix as possible for them to still be
// unique.
//
// As an example, the following keys:
// - far
// - fast
// - john
// Would be truncated to:
// - far
// - fas
// - j
pub(crate) fn truncate(keys: &[Key]) -> Vec<Key> {
    let mut out = vec![Vec::new(); keys.len()];
    // let mut out = Vec::with_capacity(keys.len());

    for i in 0..keys.len() {
        let key = &keys[i];

        // To be able to truncate a key, we must find the lowest-indexed
        // bytes where:
        // - It differs from the equivalent byte of the preceding key
        // - It differs from the equivalent byte of the next key
        //
        // Then, the larger of the two defines the boundary of a prefix
        // of the key such that it can be distinguished from both the
        // key before as well as the one after.
        let mut first_difference_before = 0;
        let mut first_difference_after = 0;

        if i != 0 {
            let (differ, fdb) = first_difference_at(key, &keys[i - 1]);
            if !differ {
                first_difference_before = key.len();
            } else {
                first_difference_before = fdb;
            }
        };

        if i != keys.len() - 1 {
            let (differ, fda) = first_difference_at(key, &keys[i + 1]);
            if !differ {
                first_difference_after = key.len();
            } else {
                first_difference_after = fda;
            }
        }

        let n = if first_difference_after > first_difference_before {
            first_difference_after
        } else {
            first_difference_before
        };

        let n = if n < key.len() {
            // We have the lowest index such that the prefix differs, so we
            // must include this one.
            // However, this would be invalid if the current key had
            // been the shorter of the two, as then the first index
            // where the two differ will already be equal to
            // len(key).
            n + 1
        } else {
            n
        };

        out[i] = key[..n].to_vec();
    }

    out
}

// first_difference_at compares two byte slices and finds the first byte where they
// differ.
//
// The first returned value indicates whether the two differ. If it is true,
// the second indicates the first index at which the two differ.
//
// If the two are of different lengths, with the shorter being a prefix of the
// longer, then the first byte of the longer is the one which is considered to
// differ.
fn first_difference_at(a: &[u8], b: &[u8]) -> (bool, usize) {
    let n = std::cmp::min(a.len(), b.len());

    for i in 0..n {
        if a[i] != b[i] {
            return (true, i);
        }
    }

    // Either the two are equal, or one is longer, in which case the first
    // difference is the first byte of the longer of the two.
    if a.len() == b.len() {
        (false, 0)
    } else {
        (true, n)
    }
}

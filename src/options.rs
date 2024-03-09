pub struct Options {
    // R is the ratio between the sizes of the sparse and dense LOUDS
    // encodings.
    //
    // The ratio governs which levels of the tree will be encoded in the
    // dense, and which ones in the sparse, encoding.
    // Let d(l) be the size of the dense encodings, from level 0 to l
    // (exclusive). Let s(l) be the size of the sparse encodings, from
    // level l (inclusive) to the full height of the tree.
    // Then the cutoff level `l`, where we switch from dense to spare encoding,
    // is chosen such that d(l) * R <= s(l).
    //
    // As such, reducing R leads to more levels being encoded as dense,
    // improving performance at the cost of space efficiency.
    //
    // The default is 64.
    pub r: u32,

    // HashBits governs the number of additional bits which will be used to
    // store parts of the hash value of the stored keys.
    //
    // Each additional hash bit will lower the false-positive rate of
    // point queries by 50%. They will not, however, assist with range
    // queries.
    //
    // The default is 4.
    pub hash_bits: u32,

    // RealBits governs the number of additional bits which will be used to
    // store parts of the key, in addition to what is stored in the
    // truncated tree.
    //
    // Each additional real bit will lower the false-positive rate of both
    // point and range queries. The exact amount by which it is lowered
    // depends on the distribution of keys.
    //
    // For the ideal case of a uniform distribution, each bit will lower it
    // by 50%. The less uniform the distribution is, the less the
    // false-positivity rate will be lowered per additional bit.
    //
    // The default is 4.
    pub real_bits: u32,

    // MemoryLimit sets the memory limit, in bytes, of the LOUDS encoding.
    //
    // The default is 256 MB
    pub memory_limit: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self::new()
    }
}

impl Options {
    pub fn new() -> Options {
        Options {
            r: 64,
            hash_bits: 4,
            real_bits: 4,
            memory_limit: 256_000_000,
        }
    }
}

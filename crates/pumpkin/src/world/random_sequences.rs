use std::collections::HashMap;

use pumpkin_util::identifier::Identifier;
use pumpkin_util::random::RandomImpl;
use pumpkin_util::random::xoroshiro128::{Xoroshiro, mix_stafford_13};

/// Computes standard MD5 (RFC 1321) hash of input bytes.
#[must_use]
pub fn md5_128(input: &[u8]) -> [u8; 16] {
    md5::compute(input).0
}

#[must_use]
pub fn seed_from_hash_of(input: &str) -> (u64, u64) {
    let digest = md5_128(input.as_bytes());
    let lo = u64::from_be_bytes(digest[0..8].try_into().unwrap());
    let hi = u64::from_be_bytes(digest[8..16].try_into().unwrap());
    (lo, hi)
}

/// A single random sequence wrapper matching vanilla `RandomSequence`.
pub struct RandomSequence {
    rng: Xoroshiro,
}

impl RandomSequence {
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self {
            rng: Xoroshiro::from_seed(seed),
        }
    }

    #[must_use]
    pub fn from_seed_and_key(seed: i64, key: Option<&str>) -> Self {
        let mut lo = (seed as u64) ^ 0x6A09_E667_F3BC_C909;
        let mut hi = lo.wrapping_add(0x9E37_79B9_7F4A_7C15);

        if let Some(k) = key {
            let (hash_lo, hash_hi) = seed_from_hash_of(k);
            lo ^= hash_lo;
            hi ^= hash_hi;
        }

        let lo = mix_stafford_13(lo);
        let hi = mix_stafford_13(hi);
        Self {
            rng: Xoroshiro::new(lo, hi),
        }
    }

    #[must_use]
    pub fn for_trade_set(profession: &str, level: i32, world_seed: i64) -> Self {
        let key = format!(
            "minecraft:trade_set/{}/level_{}",
            profession.to_ascii_lowercase(),
            level
        );
        Self::from_seed_and_key(world_seed, Some(&key))
    }

    pub fn random_between_inclusive(&mut self, min: i32, max: i32) -> i32 {
        if min >= max {
            return min;
        }
        self.rng.next_inbetween_i32(min, max)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.rng.next_i64() as u64
    }

    pub fn next_bounded_i32(&mut self, bound: i32) -> i32 {
        self.rng.next_bounded_i32(bound)
    }

    pub fn next_bool(&mut self) -> bool {
        self.rng.next_bool()
    }

    pub fn next_f32(&mut self) -> f32 {
        self.rng.next_f32()
    }

    pub fn next_f64(&mut self) -> f64 {
        self.rng.next_f64()
    }

    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = self.rng.next_bounded_i32((i + 1) as i32) as usize;
            slice.swap(i, j);
        }
    }

    pub fn choose<'a, T>(&mut self, slice: &'a [T]) -> Option<&'a T> {
        if slice.is_empty() {
            None
        } else {
            let idx = self.rng.next_bounded_i32(slice.len() as i32) as usize;
            Some(&slice[idx])
        }
    }
}

impl RandomImpl for RandomSequence {
    fn split(&mut self) -> Self {
        Self {
            rng: self.rng.split(),
        }
    }

    fn next_splitter(&mut self) -> pumpkin_util::random::RandomDeriver {
        pumpkin_util::random::RandomDeriver::Xoroshiro(self.rng.next_splitter())
    }

    fn next_i32(&mut self) -> i32 {
        self.rng.next_i32()
    }

    fn next_bounded_i32(&mut self, bound: i32) -> i32 {
        self.rng.next_bounded_i32(bound)
    }

    fn next_i64(&mut self) -> i64 {
        self.rng.next_i64()
    }

    fn next_bool(&mut self) -> bool {
        self.rng.next_bool()
    }

    fn next_f32(&mut self) -> f32 {
        self.rng.next_f32()
    }

    fn next_f64(&mut self) -> f64 {
        self.rng.next_f64()
    }

    fn next_gaussian(&mut self) -> f64 {
        self.rng.next_gaussian()
    }
}

/// Persistent/runtime manager for server random sequences.
pub struct RandomSequences {
    salt: i32,
    include_world_seed: bool,
    include_sequence_id: bool,
    sequences: HashMap<String, RandomSequence>,
}

impl Default for RandomSequences {
    fn default() -> Self {
        Self::new()
    }
}

impl RandomSequences {
    #[must_use]
    pub fn new() -> Self {
        Self {
            salt: 0,
            include_world_seed: true,
            include_sequence_id: true,
            sequences: HashMap::new(),
        }
    }

    fn create_sequence(
        sequence: &Identifier,
        world_seed: i64,
        salt: i32,
        include_world_seed: bool,
        include_sequence_id: bool,
    ) -> RandomSequence {
        let base_seed = if include_world_seed { world_seed } else { 0 };
        let seed = base_seed ^ i64::from(salt);
        let key_str = sequence.to_string();
        let key = if include_sequence_id {
            Some(key_str.as_str())
        } else {
            None
        };
        RandomSequence::from_seed_and_key(seed, key)
    }

    pub fn get_or_create(&mut self, sequence: &Identifier, world_seed: i64) -> &mut RandomSequence {
        let key = sequence.to_string();
        let salt = self.salt;
        let include_world_seed = self.include_world_seed;
        let include_sequence_id = self.include_sequence_id;
        self.sequences.entry(key).or_insert_with(|| {
            Self::create_sequence(
                sequence,
                world_seed,
                salt,
                include_world_seed,
                include_sequence_id,
            )
        })
    }

    pub fn reset(&mut self, sequence: &Identifier, world_seed: i64) {
        let key = sequence.to_string();
        let seq = Self::create_sequence(
            sequence,
            world_seed,
            self.salt,
            self.include_world_seed,
            self.include_sequence_id,
        );
        self.sequences.insert(key, seq);
    }

    pub fn reset_with_options(
        &mut self,
        sequence: &Identifier,
        world_seed: i64,
        salt: i32,
        include_world_seed: bool,
        include_sequence_id: bool,
    ) {
        let key = sequence.to_string();
        let seq = Self::create_sequence(
            sequence,
            world_seed,
            salt,
            include_world_seed,
            include_sequence_id,
        );
        self.sequences.insert(key, seq);
    }

    pub fn clear(&mut self) -> usize {
        let count = self.sequences.len();
        self.sequences.clear();
        count
    }

    pub const fn set_seed_defaults(
        &mut self,
        salt: i32,
        include_world_seed: bool,
        include_sequence_id: bool,
    ) {
        self.salt = salt;
        self.include_world_seed = include_world_seed;
        self.include_sequence_id = include_sequence_id;
    }

    #[must_use]
    pub fn get_sequence_keys(&self) -> Vec<String> {
        self.sequences.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md5() {
        let empty = md5_128(b"");
        assert_eq!(
            hex::encode(empty),
            "d41d8cd98f00b204e9800998ecf8427e"
        );
        let abc = md5_128(b"abc");
        assert_eq!(
            hex::encode(abc),
            "900150983cd24fb0d6963f7d28e17f72"
        );
    }
}

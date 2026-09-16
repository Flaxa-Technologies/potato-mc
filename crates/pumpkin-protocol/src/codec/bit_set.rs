use std::io::Read;
use std::io::Write;

use crate::ReadingError;
use crate::WritingError;
use crate::codec::var_int::VarInt;
use crate::ser::NetworkReadExt;
use crate::ser::NetworkWriteExt;
use pumpkin_util::version::JavaMinecraftVersion;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BitSet(pub Box<[i64]>);

impl BitSet {
    #[must_use]
    pub fn from_u64(val: u64) -> Self {
        Self(Box::new([val as i64]))
    }

    #[must_use]
    pub fn from_i64(val: i64) -> Self {
        Self(Box::new([val]))
    }

    #[must_use]
    pub fn from_longs(longs: Vec<i64>) -> Self {
        Self(longs.into_boxed_slice())
    }

    #[must_use]
    pub fn as_u64(&self) -> u64 {
        self.0.first().copied().unwrap_or(0) as u64
    }

    #[must_use]
    pub fn as_i64(&self) -> i64 {
        self.0.first().copied().unwrap_or(0)
    }

    #[must_use]
    pub fn get_bit(&self, index: usize) -> bool {
        let word_idx = index / 64;
        let bit_idx = index % 64;
        self.0
            .get(word_idx)
            .is_some_and(|&w| (w & (1i64 << bit_idx)) != 0)
    }

    pub fn set_bit(&mut self, index: usize, val: bool) {
        let word_idx = index / 64;
        let bit_idx = index % 64;
        if word_idx >= self.0.len() {
            let mut vec = self.0.to_vec();
            vec.resize(word_idx + 1, 0);
            self.0 = vec.into_boxed_slice();
        }
        if val {
            self.0[word_idx] |= 1i64 << bit_idx;
        } else {
            self.0[word_idx] &= !(1i64 << bit_idx);
        }
    }

    #[must_use]
    pub fn count_ones(&self) -> u32 {
        self.0.iter().map(|&w| (w as u64).count_ones()).sum()
    }

    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut words_in_use = self.0.len();
        while words_in_use > 0 && self.0[words_in_use - 1] == 0 {
            words_in_use -= 1;
        }
        if words_in_use == 0 {
            return Vec::new();
        }

        let mut byte_len = 8 * (words_in_use - 1);
        let mut x = self.0[words_in_use - 1] as u64;
        while x != 0 {
            byte_len += 1;
            x >>= 8;
        }

        let mut bytes = Vec::with_capacity(byte_len);
        for i in 0..byte_len {
            let word_idx = i / 8;
            let byte_in_word = i % 8;
            let val = ((self.0[word_idx] as u64) >> (byte_in_word * 8)) as u8;
            bytes.push(val);
        }
        bytes
    }

    pub fn encode_for_version(
        &self,
        write: &mut impl Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        if *version >= JavaMinecraftVersion::V_26_3 {
            let bytes = self.to_bytes();
            write.write_var_int(&VarInt(bytes.len() as i32))?;
            write.write_slice(&bytes)?;
            Ok(())
        } else {
            self.encode(write)
        }
    }

    pub fn encode(&self, write: &mut impl Write) -> Result<(), WritingError> {
        write.write_var_int(&self.0.len().try_into().map_err(|_| {
            WritingError::Message(format!("{} isn't representable as a VarInt", self.0.len()))
        })?)?;

        for b in &self.0 {
            write.write_i64_be(*b)?;
        }

        Ok(())
    }

    pub fn decode(read: &mut impl Read) -> Result<Self, ReadingError> {
        // Read length
        let length = read.get_var_int()?;
        let mut array: Vec<i64> = Vec::with_capacity(length.0 as usize);
        for _ in 0..length.0 {
            let long = read.get_i64_be()?;
            array.push(long);
        }
        Ok(Self(array.into_boxed_slice()))
    }
}

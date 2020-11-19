use alloc::vec::{IntoIter, Vec};
use core::{
    iter::FromIterator,
    mem,
    ops::{Deref, Index, Range, RangeFrom, RangeFull, RangeTo},
    slice,
};

use datasize::DataSize;
use serde::{Deserialize, Serialize};

use super::{Error, FromBytes, ToBytes};
use crate::{CLType, CLTyped};

/// A newtype wrapper for bytes that has efficient serialization routines.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Default, Hash, Serialize, Deserialize)]
pub struct Bytes(Vec<u8>);

impl Bytes {
    /// Constructs a new, empty vector of bytes.
    pub fn new() -> Bytes {
        Bytes::default()
    }

    /// Returns reference to inner container.
    #[inline]
    pub fn inner_bytes(&self) -> &Vec<u8> {
        &self.0
    }

    /// Extracts a slice containing the entire vector.
    pub fn as_slice(&self) -> &[u8] {
        self
    }
}

impl Deref for Bytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl From<Vec<u8>> for Bytes {
    fn from(vec: Vec<u8>) -> Self {
        Self(vec)
    }
}

impl From<Bytes> for Vec<u8> {
    fn from(bytes: Bytes) -> Self {
        bytes.0
    }
}

impl From<&[u8]> for Bytes {
    fn from(bytes: &[u8]) -> Self {
        Self(bytes.to_vec())
    }
}

impl CLTyped for Bytes {
    fn cl_type() -> CLType {
        <Vec<u8>>::cl_type()
    }
}

impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl ToBytes for Bytes {
    #[inline(always)]
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        super::serialize_bytes(&self.0)
    }

    #[inline(always)]
    fn into_bytes(self) -> Result<Vec<u8>, Error> {
        super::serialize_bytes(&self.0)
    }

    #[inline(always)]
    fn serialized_length(&self) -> usize {
        super::bytes_serialized_length(&self.0)
    }
}

impl FromBytes for Bytes {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), super::Error> {
        let (size, remainder) = u32::from_bytes(bytes)?;
        let (result, remainder) = super::safe_split_at(remainder, size as usize)?;
        Ok((Bytes(result.to_vec()), remainder))
    }

    fn from_vec(stream: Vec<u8>) -> Result<(Self, Vec<u8>), Error> {
        let (size, mut stream) = u32::from_vec(stream)?;

        if size as usize > stream.len() {
            Err(Error::EarlyEndOfStream)
        } else {
            let remainder = stream.split_off(size as usize);
            Ok((Bytes(stream), remainder))
        }
    }
}

impl Index<usize> for Bytes {
    type Output = u8;

    fn index(&self, index: usize) -> &u8 {
        let Bytes(ref dat) = self;
        &dat[index]
    }
}

impl Index<Range<usize>> for Bytes {
    type Output = [u8];

    fn index(&self, index: Range<usize>) -> &[u8] {
        let &Bytes(ref dat) = self;
        &dat[index]
    }
}

impl Index<RangeTo<usize>> for Bytes {
    type Output = [u8];

    fn index(&self, index: RangeTo<usize>) -> &[u8] {
        let &Bytes(ref dat) = self;
        &dat[index]
    }
}

impl Index<RangeFrom<usize>> for Bytes {
    type Output = [u8];

    fn index(&self, index: RangeFrom<usize>) -> &[u8] {
        let &Bytes(ref dat) = self;
        &dat[index]
    }
}

impl Index<RangeFull> for Bytes {
    type Output = [u8];

    fn index(&self, _: RangeFull) -> &[u8] {
        let &Bytes(ref dat) = self;
        &dat[..]
    }
}

impl FromIterator<u8> for Bytes {
    #[inline]
    fn from_iter<I: IntoIterator<Item = u8>>(iter: I) -> Bytes {
        let vec = Vec::from_iter(iter);
        Bytes(vec)
    }
}

impl<'a> IntoIterator for &'a Bytes {
    type Item = &'a u8;

    type IntoIter = slice::Iter<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl IntoIterator for Bytes {
    type Item = u8;

    type IntoIter = IntoIter<u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl DataSize for Bytes {
    const IS_DYNAMIC: bool = true;

    const STATIC_HEAP_SIZE: usize = 0;

    fn estimate_heap_size(&self) -> usize {
        self.0.capacity() * mem::size_of::<u8>()
    }
}

#[cfg(test)]
mod tests {
    use crate::bytesrepr::{self, Error, FromBytes, ToBytes, U32_SERIALIZED_LENGTH};
    use alloc::vec::Vec;

    use super::Bytes;

    #[test]
    fn vec_u8_from_bytes() {
        let data: Bytes = vec![1, 2, 3, 4, 5].into();
        let data_bytes = data.to_bytes().unwrap();
        assert!(Bytes::from_bytes(&data_bytes[..U32_SERIALIZED_LENGTH / 2]).is_err());
        assert!(Bytes::from_bytes(&data_bytes[..U32_SERIALIZED_LENGTH]).is_err());
        assert!(Bytes::from_bytes(&data_bytes[..U32_SERIALIZED_LENGTH + 2]).is_err());
    }

    #[test]
    fn should_serialize_deserialize_bytes() {
        let data: Bytes = vec![1, 2, 3, 4, 5].into();
        bytesrepr::test_serialization_roundtrip(&data);
    }

    #[test]
    fn should_fail_to_serialize_deserialize_malicious_bytes() {
        let data: Bytes = vec![1, 2, 3, 4, 5].into();
        let mut serialized = data.to_bytes().expect("should serialize data");
        serialized = serialized[..serialized.len() - 1].to_vec();
        let res: Result<(_, &[u8]), Error> = Bytes::from_bytes(&serialized);
        assert_eq!(res.unwrap_err(), Error::EarlyEndOfStream);
    }

    #[test]
    fn should_serialize_deserialize_bytes_and_keep_rem() {
        let data: Bytes = vec![1, 2, 3, 4, 5].into();
        let expected_rem: Vec<u8> = vec![6, 7, 8, 9, 10];
        let mut serialized = data.to_bytes().expect("should serialize data");
        serialized.extend(&expected_rem);
        let (deserialized, rem): (Bytes, &[u8]) =
            FromBytes::from_bytes(&serialized).expect("should deserialize data");
        assert_eq!(data, deserialized);
        assert_eq!(&rem, &expected_rem);
    }
}

#[cfg(test)]
pub mod gens {
    use super::Bytes;
    use proptest::{
        collection::{vec, SizeRange},
        prelude::*,
    };

    pub fn bytes_arb(size: impl Into<SizeRange>) -> impl Strategy<Value = Bytes> {
        vec(any::<u8>(), size).prop_map(|vec| Bytes::from(vec))
    }
}

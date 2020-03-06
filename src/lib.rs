#[macro_use]
extern crate rental;
extern crate stable_deref_trait;

use stable_deref_trait::{CloneStableDeref, StableDeref};
use std::io;
use std::ops::Deref;
use std::sync::Arc;

rental! {
    mod rental_impl {
        use ::BoxStableDeref;

        #[rental(clone, deref_suffix)]
        pub(crate) struct OwnedReader {
            head: BoxStableDeref,
            data: & 'head [u8],
        }
    }
}

#[derive(Clone)]
struct BoxStableDeref(Arc<Deref<Target = [u8]>>);

impl BoxStableDeref {
    fn new<T: Deref<Target = [u8]> + StableDeref + 'static>(inner: T) -> BoxStableDeref {
        BoxStableDeref(Arc::new(inner))
    }
}

unsafe impl StableDeref for BoxStableDeref {}
unsafe impl CloneStableDeref for BoxStableDeref {}

impl Deref for BoxStableDeref {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.0.deref()
    }
}

#[derive(Clone)]
pub struct OwnedRead {
    inner: rental_impl::OwnedReader,
}

impl OwnedRead {
    pub fn new<T: StableDeref + Deref<Target = [u8]> + 'static>(data: T) -> OwnedRead {
        let box_stable_deref = BoxStableDeref::new(data);
        let inner = rental_impl::OwnedReader::new(box_stable_deref, |arr| arr);
        OwnedRead { inner }
    }

    fn as_slice(&self) -> &[u8] {
        self.inner.deref()
    }

    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }

    pub fn clip(&mut self, clip_len: usize) {
        rental_impl::OwnedReader::rent_mut(&mut self.inner, |arr| *arr = &arr[..clip_len]);
    }

    pub fn advance(&mut self, advance_len: usize) {
        rental_impl::OwnedReader::rent_mut(&mut self.inner, |arr| *arr = &arr[advance_len..]);
    }

    pub fn slice_from(&self, start: usize) -> &[u8] {
        &self.as_slice()[start..]
    }

    pub fn get(&self, idx: usize) -> u8 {
        self.as_slice()[idx]
    }
}

impl io::Read for OwnedRead {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read_len = {
            let data = self.as_slice();
            if data.len() >= buf.len() {
                let buf_len = buf.len();
                buf.copy_from_slice(&data[..buf_len]);
                buf.len()
            } else {
                let data_len = data.len();
                buf[..data_len].copy_from_slice(data);
                data_len
            }
        };
        self.advance(read_len);
        Ok(read_len)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        let read_len = {
            let data = self.as_slice();
            buf.extend(data);
            data.len()
        };
        self.advance(read_len);
        Ok(read_len)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        let read_len = self.read(buf)?;
        if read_len == buf.len() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "failed to fill whole buffer",
            ))
        }
    }
}

impl AsRef<[u8]> for OwnedRead {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

#[cfg(test)]
mod tests {
    use super::OwnedRead;
    use std::io::Read;

    #[test]
    fn test_read() {
        let mut source_reader = OwnedRead::new((0u8..100u8).collect::<Vec<_>>());
        let mut buffer = vec![0u8; 5];
        assert!(source_reader.read_exact(&mut buffer).is_ok());
        assert_eq!(buffer, &[0u8, 1u8, 2u8, 3u8, 4u8]);

        let mut cloned_reader = source_reader.clone();
        assert!(cloned_reader.read_exact(&mut buffer).is_ok());
        assert_eq!(buffer, &[5u8, 6u8, 7u8, 8u8, 9u8]);
    }
}

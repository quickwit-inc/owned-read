use stable_deref_trait::StableDeref;
use std::io;
use std::mem;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Clone)]
pub struct OwnedRead {
    data: &'static [u8],
    box_stable_deref: Arc<dyn Deref<Target = [u8]>>,
}

impl OwnedRead {
    pub fn new<T: StableDeref + Deref<Target = [u8]> + 'static>(data_holder: T) -> OwnedRead {
        let box_stable_deref = Arc::new(data_holder);
        let data = unsafe { mem::transmute::<_, &'static [u8]>(box_stable_deref.deref().deref()) };
        OwnedRead {
            box_stable_deref,
            data,
        }
    }
    fn as_slice(&self) -> &[u8] {
        self.data
    }
    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }
    pub fn clip(&mut self, clip_len: usize) {
        self.data = &self.data[..clip_len]
    }
    pub fn advance(&mut self, advance_len: usize) {
        self.data = &self.data[advance_len..]
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
        if read_len != buf.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "failed to fill whole buffer",
            ));
        }
        Ok(())
    }
}
impl AsRef<[u8]> for OwnedRead {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

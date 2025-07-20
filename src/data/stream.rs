use core::{cmp::Ord, fmt::Debug};
use std::io::{Read, Write};

pub trait ReadWrite: Read + Write + Debug {}
impl<T: Read + Write + Debug> ReadWrite for T {}

#[derive(Debug)]
pub struct DataStream<'a> {
    remaining_size: usize,
    inner: &'a mut (dyn ReadWrite + 'a),
}

impl<'a> DataStream<'a> {
        pub fn new(inner: &'a mut dyn ReadWrite, packet_size: usize) -> Self {
            Self {
                inner,
                remaining_size: packet_size,
            }
        }

    pub fn remaining_size(&self) -> usize {
        self.remaining_size
    }
}

impl<'a> Read for DataStream<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let len = buf.len();
        let buf = &mut buf[..self.remaining_size.min(len)];
        let n = self.inner.read(buf)?;
        self.remaining_size -= n;
        Ok(n)
    }
}

impl<'a> Write for DataStream<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

use std::io::{self, Read};

struct SmallBuffer<const SIZE: usize> {
    buf: [u8; SIZE],
    len: usize,
}

impl<const SIZE: usize> Default for SmallBuffer<SIZE> {
    fn default() -> Self {
        Self {
            buf: [0u8; SIZE],
            len: 0,
        }
    }
}

impl<const SIZE: usize> SmallBuffer<SIZE> {
    fn new() -> Self {
        Self::default()
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn overwrite_char_utf8(&mut self, c: char) {
        let s = c.encode_utf8(&mut self.buf);
        assert!(s.len() <= SIZE);
        self.len = s.len();
    }
}

impl<const SIZE: usize> Read for SmallBuffer<SIZE> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = std::cmp::min(buf.len(), self.len);
        buf[..len].clone_from_slice(&self.buf[..len]);
        self.buf.copy_within(len.., 0);
        self.len -= len;
        Ok(len)
    }
}

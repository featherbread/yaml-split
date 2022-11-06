use std::io::{self, BufRead, Cursor, Read, Write};

const MAX_UTF8_ENCODED_LEN: usize = 4;

pub struct UTF8Encoder<S>
where
    S: Iterator<Item = io::Result<char>>,
{
    source: S,
    remainder: Cursor<[u8; MAX_UTF8_ENCODED_LEN]>,
    remainder_len: usize,
}

impl<S> UTF8Encoder<S>
where
    S: Iterator<Item = io::Result<char>>,
{
    pub fn new(source: S) -> Self {
        Self {
            source,
            remainder: Cursor::new([0u8; MAX_UTF8_ENCODED_LEN]),
            remainder_len: 0,
        }
    }
}

impl<S> Read for UTF8Encoder<S>
where
    S: Iterator<Item = io::Result<char>>,
{
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        let mut written = 0;

        // First, emit the remainder of any character that a previous read could
        // not fully emit.
        if self.remainder_len > 0 {
            let len = std::cmp::min(buf.len(), self.remainder_len);
            self.remainder.read_exact(&mut buf[..len])?;
            buf = &mut buf[len..];
            written += len;
            self.remainder_len -= len;
            if self.remainder_len > 0 {
                return Ok(written);
            }
        }

        // Second, emit as much as we can directly into the destination buffer.
        while buf.len() >= MAX_UTF8_ENCODED_LEN {
            let ch = match self.source.next() {
                Some(Ok(ch)) => ch,
                Some(Err(err)) => return Err(err),
                None => return Ok(written),
            };
            let len = ch.encode_utf8(buf).len();
            buf = &mut buf[len..];
            written += len;
        }

        // Finally, emit as much as we can into the destination buffer's
        // remaining space, storing the remainder of any character that we
        // cannot fully emit at this time.
        while !buf.is_empty() {
            let ch = match self.source.next() {
                Some(Ok(ch)) => ch,
                Some(Err(err)) => return Err(err),
                None => return Ok(written),
            };

            let mut tmp = [0u8; MAX_UTF8_ENCODED_LEN];
            let char_len = ch.encode_utf8(&mut tmp).len();

            let emit_len = std::cmp::min(char_len, buf.len());
            buf[..emit_len].copy_from_slice(&tmp[..emit_len]);
            buf = &mut buf[emit_len..];
            written += emit_len;

            if buf.is_empty() {
                self.remainder.set_position(0);
                self.remainder.write_all(&tmp[emit_len..char_len])?;
                self.remainder_len = char_len - emit_len;
            }
        }

        Ok(written)
    }
}

pub enum Endianness {
    BE,
    LE,
}

impl Endianness {
    fn decode_u32(&self, buf: [u8; 4]) -> u32 {
        match self {
            Endianness::BE => u32::from_be_bytes(buf),
            Endianness::LE => u32::from_le_bytes(buf),
        }
    }
}

pub struct UTF32Decoder<R>
where
    R: BufRead,
{
    source: R,
    endianness: Endianness,
}

impl<R> UTF32Decoder<R>
where
    R: BufRead,
{
    pub fn new(source: R, endianness: Endianness) -> Self {
        Self { source, endianness }
    }
}

impl<R> Iterator for UTF32Decoder<R>
where
    R: BufRead,
{
    type Item = io::Result<char>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.source.fill_buf() {
            Ok(buf) if buf.is_empty() => return None,
            Err(err) => return Some(Err(err)),
            _ => {}
        };

        let mut next = [0u8; 4];
        if let Err(err) = self.source.read_exact(&mut next) {
            return Some(Err(err));
        }

        Some(Ok(
            char::from_u32(self.endianness.decode_u32(next)).unwrap_or(char::REPLACEMENT_CHARACTER)
        ))
    }
}

use std::io::{self, BufRead, Cursor, Read, Write};

const MAX_UTF8_ENCODED_LEN: usize = 4;

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

pub struct UTF32Converter<R>
where
    R: BufRead,
{
    source: R,
    endianness: Endianness,
    remainder: Cursor<Vec<u8>>,
}

impl<R: BufRead> UTF32Converter<R> {
    pub fn new(source: R, endianness: Endianness) -> Self {
        Self {
            source,
            endianness,
            remainder: Cursor::new(Vec::new()),
        }
    }
}

impl<R> Read for UTF32Converter<R>
where
    R: BufRead,
{
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        let mut written = 0;

        if !self.remainder.get_ref().is_empty() {
            let len = std::cmp::min(buf.len(), self.remainder.get_ref().len());
            self.remainder.read_exact(&mut buf[..len])?;
            buf = &mut buf[len..];
            written += len;
            if !self.remainder.get_ref().is_empty() {
                return Ok(written);
            }
        }

        while buf.len() >= MAX_UTF8_ENCODED_LEN {
            let ch = match self.read_next_char()? {
                Some(ch) => ch,
                None => return Ok(written),
            };
            let len = ch.encode_utf8(buf).len();
            buf = &mut buf[len..];
            written += len;
        }

        while !buf.is_empty() {
            let ch = match self.read_next_char()? {
                Some(ch) => ch,
                None => return Ok(written),
            };

            let mut tmp = [0u8; MAX_UTF8_ENCODED_LEN];
            let char_len = ch.encode_utf8(&mut tmp).len();

            let emit_len = std::cmp::min(char_len, buf.len());
            buf[..emit_len].copy_from_slice(&tmp[..emit_len]);
            buf = &mut buf[emit_len..];
            written += emit_len;

            if emit_len < char_len {
                self.remainder.write_all(&tmp[emit_len..char_len])?;
            }
        }

        Ok(written)
    }
}

impl<R> UTF32Converter<R>
where
    R: BufRead,
{
    fn read_next_char(&mut self) -> io::Result<Option<char>> {
        if self.source.fill_buf()?.is_empty() {
            return Ok(None);
        }

        let mut next = [0u8; 4];
        self.source.read_exact(&mut next)?;
        Ok(Some(
            char::from_u32(self.endianness.decode_u32(next)).unwrap_or(char::REPLACEMENT_CHARACTER),
        ))
    }
}

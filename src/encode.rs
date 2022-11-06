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
    Big,
    Little,
}

impl Endianness {
    fn decode_u16(&self, buf: [u8; 2]) -> u16 {
        match self {
            Endianness::Big => u16::from_be_bytes(buf),
            Endianness::Little => u16::from_le_bytes(buf),
        }
    }

    fn decode_u32(&self, buf: [u8; 4]) -> u32 {
        match self {
            Endianness::Big => u32::from_be_bytes(buf),
            Endianness::Little => u32::from_le_bytes(buf),
        }
    }
}

pub struct UTF16Decoder<R>
where
    R: BufRead,
{
    source: R,
    endianness: Endianness,
    buf: Option<u16>,
}

impl<R> UTF16Decoder<R>
where
    R: BufRead,
{
    pub fn new(source: R, endianness: Endianness) -> Self {
        Self {
            source,
            endianness,
            buf: None,
        }
    }
}

impl<R> Iterator for UTF16Decoder<R>
where
    R: BufRead,
{
    type Item = io::Result<char>;

    fn next(&mut self) -> Option<Self::Item> {
        let lead = match self.buf.take() {
            Some(u) => u,
            None => match self.next_u16() {
                Ok(Some(u)) => u,
                Ok(None) => return None,
                Err(err) => return Some(Err(err)),
            },
        };

        if !(0xD800..=0xDFFF).contains(&lead) {
            // SAFETY: This is not a UTF-16 surrogate, which means that the u16
            // code unit directly encodes the desired code point.
            return Some(Ok(unsafe { char::from_u32_unchecked(lead as u32) }));
        }

        if lead >= 0xDC00 {
            // Invalid: a UTF-16 trailing surrogate with no leading surrogate.
            return Some(Ok(char::REPLACEMENT_CHARACTER));
        }

        let trail = match self.next_u16() {
            Ok(Some(u)) => u,
            Ok(None) => return Some(Ok(char::REPLACEMENT_CHARACTER)),
            Err(err) => return Some(Err(err)),
        };
        if !(0xDC00..=0xDFFF).contains(&trail) {
            // Invalid: we needed a trailing surrogate and didn't get one. We'll
            // try to decode this as a leading code unit on the next iteration.
            self.buf = Some(trail);
            return Some(Ok(char::REPLACEMENT_CHARACTER));
        }

        // At this point, we are confident that we have valid leading and
        // trailing surrogates, and can decode them into the correct code point.
        let ch = 0x1_0000 + (((lead - 0xD800) as u32) << 10 | (trail - 0xDC00) as u32);
        // SAFETY: We have confirmed that the surrogate pair is valid.
        Some(Ok(unsafe { char::from_u32_unchecked(ch as u32) }))
    }
}

impl<R> UTF16Decoder<R>
where
    R: BufRead,
{
    fn next_u16(&mut self) -> io::Result<Option<u16>> {
        match self.source.fill_buf() {
            Ok(buf) if buf.is_empty() => return Ok(None),
            Err(err) => return Err(err),
            _ => {}
        };

        let mut buf = [0u8; 2];
        self.source.read_exact(&mut buf)?;
        Ok(Some(self.endianness.decode_u16(buf)))
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

use std::io::{self, BufRead, Read};
use std::mem::MaybeUninit;
use std::pin::Pin;

use unsafe_libyaml::*; // It's called a proof-of-concept, okay?

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
        (&mut buf[..len]).clone_from_slice(&self.buf[..len]);
        self.buf.copy_within(len.., 0);
        self.len -= len;
        return Ok(len);
    }
}

const YAML_ENCODING_PREFIX_LEN: usize = 4;
const MAX_UTF8_CHAR_BYTES: usize = 4;

struct YAMLStreamEncoder<R>
where
    R: Read,
{
    source: R,
    detect_prefix: SmallBuffer<YAML_ENCODING_PREFIX_LEN>,
    output_prefix: SmallBuffer<MAX_UTF8_CHAR_BYTES>,
}

enum UTF32Endianness {
    BE,
    LE,
}

impl UTF32Endianness {
    fn decode(&self, buf: [u8; 4]) -> u32 {
        match self {
            Self::BE => u32::from_be_bytes(buf),
            Self::LE => u32::from_le_bytes(buf),
        }
    }
}

struct UTF32StreamEncoder<R>
where
    R: BufRead,
{
    source: R,
    endianness: UTF32Endianness,
}

impl<R> UTF32StreamEncoder<R>
where
    R: BufRead,
{
    fn new(source: R, endianness: UTF32Endianness) -> Self {
        Self { source, endianness }
    }
}

impl<R> Iterator for UTF32StreamEncoder<R>
where
    R: BufRead,
{
    type Item = Result<char, io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.source.fill_buf() {
            Ok(buf) if buf.len() == 0 => return None,
            Err(err) => return Some(Err(err)),
            _ => {}
        };

        let mut buf = [0u8; 4];
        if let Err(err) = self.source.read_exact(&mut buf) {
            return Some(Err(err));
        }

        match char::from_u32(self.endianness.decode(buf.try_into().unwrap())) {
            Some(ch) => Some(Ok(ch)),
            None => Some(Err(io::ErrorKind::InvalidInput.into())),
        }
    }
}

pub struct Splitter {
    parser: Pin<Box<yaml_parser_t>>,
}

impl Splitter {
    pub fn new() -> Self {
        let mut parser_uninit = Box::pin(MaybeUninit::<yaml_parser_t>::zeroed());

        // SAFETY: libyaml is assumed to be correct.
        if unsafe { yaml_parser_initialize(parser_uninit.as_mut_ptr()).fail } {
            panic!("failed to initialize YAML parser");
        }

        // SAFETY: MaybeUninit<T> is guaranteed by the standard library to have
        // the same size, alignment, and ABI as T. This usage is roughly similar
        // to the "Initializing an array element-by-element" example in the
        // MaybeUninit docs.
        let parser = unsafe {
            std::mem::transmute::<Pin<Box<MaybeUninit<yaml_parser_t>>>, Pin<Box<yaml_parser_t>>>(
                parser_uninit,
            )
        };
        Self { parser }
    }
}

impl Drop for Splitter {
    fn drop(&mut self) {
        unsafe { yaml_parser_delete(self.parser.as_mut().get_unchecked_mut()) }
    }
}

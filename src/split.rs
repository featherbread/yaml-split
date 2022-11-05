use std::io::{self, Read};
use std::mem::MaybeUninit;
use std::pin::Pin;

use unsafe_libyaml::*; // It's called a proof-of-concept, okay?

const MAX_UTF8_CHAR_BYTES: usize = 4;

struct CharBuffer {
    buf: [u8; MAX_UTF8_CHAR_BYTES],
    len: usize,
}

impl Default for CharBuffer {
    fn default() -> Self {
        Self {
            buf: [0u8; MAX_UTF8_CHAR_BYTES],
            len: 0,
        }
    }
}

impl CharBuffer {
    fn new() -> Self {
        Self::default()
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn put(&mut self, c: char) {
        let s = c.encode_utf8(&mut self.buf);
        debug_assert!(s.len() <= MAX_UTF8_CHAR_BYTES);
        self.len = s.len();
    }
}

impl Read for CharBuffer {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = std::cmp::min(buf.len(), self.len);
        (&mut buf[..len]).clone_from_slice(&self.buf[..len]);
        self.buf.copy_within(len.., 0);
        self.len -= len;
        return Ok(len);
    }
}

enum YAMLEncoding {
    UTF8,
    UTF16BE,
    UTF16LE,
    UTF32BE,
    UTF32LE,
}

struct YAMLStreamEncoder<R>
where
    R: Read,
{
    source: R,
    encoding: Option<YAMLEncoding>,
    input_buf: Vec<u8>,
    output_buf: CharBuffer,
}

impl<R> YAMLStreamEncoder<R>
where
    R: Read,
{
    fn new(source: R) -> Self {
        Self {
            source,
            input_buf: Vec::new(),
            encoding: None,
            output_buf: CharBuffer::default(),
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

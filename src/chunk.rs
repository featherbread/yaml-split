use std::error::Error;
use std::ffi::{c_void, CStr};
use std::fmt::Display;
use std::io::{self, Read};
use std::mem::{self, MaybeUninit};
use std::ops::Deref;
use std::ptr::NonNull;

use unsafe_libyaml::*;

pub struct Chunker<R>
where
    R: Read,
{
    parser: *mut yaml_parser_t,
    read_state: *mut ReadState<R>,
}

struct ReadState<R>
where
    R: Read,
{
    reader: ChunkReader<R>,
    error: Option<io::Error>,
}

impl<R> Chunker<R>
where
    R: Read,
{
    pub fn new(reader: R) -> Self {
        // SAFETY: libyaml code is assumed to be correct. To avoid leaking
        // memory, we don't unbox the pointer until after the panic attempt.
        let parser = unsafe {
            let mut parser = Box::new(MaybeUninit::<yaml_parser_t>::uninit());
            if yaml_parser_initialize(parser.as_mut_ptr()).fail {
                panic!("out of memory for libyaml parser initialization");
            }
            Box::into_raw(parser).cast::<yaml_parser_t>()
        };

        let read_state = Box::into_raw(Box::new(ReadState {
            reader: ChunkReader::new(reader),
            error: None,
        }));
        // SAFETY: libyaml code is assumed to be correct.
        unsafe {
            yaml_parser_set_encoding(parser, YAML_UTF8_ENCODING);
            yaml_parser_set_input(parser, Self::read_callback, read_state as *mut c_void);
        }

        Self { parser, read_state }
    }

    unsafe fn read_callback(
        read_state: *mut c_void,
        buffer: *mut u8,
        size: u64,
        size_read: *mut u64,
    ) -> i32 {
        const READ_FAILURE: i32 = 0;
        const READ_SUCCESS: i32 = 1;

        // SAFETY: libyaml code is assumed to be correct, in that it passes us
        // the data pointer we originally provided.
        let read_state = read_state.cast::<ReadState<R>>();

        // TODO: libyaml code is assumed to pass us a valid buffer of the
        // provided size. However, it does seem to malloc() this buffer with no
        // further initialization of its own. We arguably aren't violating any
        // type-level invariants here, since a u8 can represent any byte, but at
        // best this is playing very fast and loose with the definition of
        // "properly initialized."
        let buf = std::slice::from_raw_parts_mut(buffer, size as usize);

        match (*read_state).reader.read(buf) {
            Ok(len) => {
                *size_read = len as u64;
                (*read_state).error = None;
                READ_SUCCESS
            }
            Err(err) => {
                (*read_state).error = Some(err);
                READ_FAILURE
            }
        }
    }
}

impl<R> Iterator for Chunker<R>
where
    R: Read,
{
    type Item = io::Result<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // SAFETY: We properly initialized self.parser and self.read_state
            // when the Chunker was constructed.
            let event = unsafe {
                match Event::from_parser(self.parser) {
                    Ok(event) => event,
                    Err(()) => {
                        return if let Some(err) = (*self.read_state).error.take() {
                            Some(Err(err))
                        } else {
                            Some(Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                ParserError::from_parser(self.parser),
                            )))
                        }
                    }
                }
            };

            match event.type_ {
                YAML_STREAM_END_EVENT => return None,
                YAML_DOCUMENT_START_EVENT => {
                    let pos = event.start_mark.index;
                    // SAFETY: We properly initialized self.read_state when the
                    // Chunker was constructed.
                    unsafe { (*self.read_state).reader.trim_to_start(pos) };
                }
                YAML_DOCUMENT_END_EVENT => {
                    let pos = event.end_mark.index;
                    // SAFETY: We properly initialized self.read_state when the
                    // Chunker was constructed.
                    let chunk = unsafe { (*self.read_state).reader.take_to_end(pos) };
                    return Some(Ok(chunk));
                }
                _ => {}
            };
        }
    }
}

impl<R> Drop for Chunker<R>
where
    R: Read,
{
    fn drop(&mut self) {
        // SAFETY: libyaml code is assumed to be correct. Both of the raw
        // pointers were originally obtained from Boxes.
        unsafe {
            yaml_parser_delete(self.parser);
            drop(Box::from_raw(self.parser));
            drop(Box::from_raw(self.read_state));
        }
    }
}

struct Event(*mut yaml_event_t);

impl Event {
    unsafe fn from_parser(parser: *mut yaml_parser_t) -> Result<Event, ()> {
        let mut event = Box::new(MaybeUninit::<yaml_event_t>::uninit());
        if yaml_parser_parse(parser, event.as_mut_ptr()).fail {
            return Err(());
        }
        Ok(Event(Box::into_raw(event).cast::<yaml_event_t>()))
    }
}

impl Deref for Event {
    type Target = yaml_event_t;

    fn deref(&self) -> &Self::Target {
        // SAFETY: We never expose the raw pointer externally, so there should
        // be no opportunity to violate aliasing rules.
        unsafe { &*self.0 }
    }
}

impl Drop for Event {
    fn drop(&mut self) {
        unsafe {
            yaml_event_delete(self.0);
            drop(Box::from_raw(self.0));
        }
    }
}

#[derive(Debug)]
struct ParserError {
    problem: Option<LocatedError>,
    context: Option<LocatedError>,
}

impl ParserError {
    unsafe fn from_parser(parser: *mut yaml_parser_t) -> Self {
        Self {
            problem: NonNull::new((*parser).problem as *mut i8).map(|problem| {
                LocatedError::from_parts(
                    problem.as_ptr().cast(),
                    (*parser).problem_mark,
                    Some((*parser).problem_offset),
                )
            }),
            context: NonNull::new((*parser).context as *mut i8).map(|context| {
                LocatedError::from_parts(context.as_ptr().cast(), (*parser).context_mark, None)
            }),
        }
    }
}

impl Error for ParserError {}

impl Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.problem {
            None => f.write_str("unknown libyaml error"),
            Some(problem) => match &self.context {
                None => Display::fmt(problem, f),
                Some(context) => write!(f, "{}, {}", problem, context),
            },
        }
    }
}

#[derive(Debug)]
struct LocatedError {
    description: String,
    pos: u64,
    line: u64,
    column: u64,
}

impl LocatedError {
    unsafe fn from_parts(
        description: *const i8,
        mark: yaml_mark_t,
        override_pos: Option<u64>,
    ) -> Self {
        Self {
            description: CStr::from_ptr(description).to_string_lossy().into_owned(),
            line: mark.line + 1,
            column: mark.column + 1,
            pos: if mark.index > 0 {
                mark.index
            } else {
                override_pos.unwrap_or(0)
            },
        }
    }
}

impl Display for LocatedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line == 1 && self.column == 1 {
            write!(f, "{} at position {}", self.description, self.pos)
        } else {
            write!(
                f,
                "{} at line {} column {}",
                self.description, self.line, self.column,
            )
        }
    }
}

struct ChunkReader<R>
where
    R: Read,
{
    reader: R,
    capture: Vec<u8>,
    capture_start_pos: u64,
}

impl<R> ChunkReader<R>
where
    R: Read,
{
    fn new(reader: R) -> Self {
        Self {
            reader,
            capture: vec![],
            capture_start_pos: 0,
        }
    }

    fn trim_to_start(&mut self, pos: u64) {
        let excess_start_len = (pos - self.capture_start_pos) as usize;
        self.capture_start_pos = pos;
        self.capture.drain(..excess_start_len);
    }

    fn take_to_end(&mut self, pos: u64) -> Vec<u8> {
        let take_len = (pos - self.capture_start_pos) as usize;
        let tail = self.capture.split_off(take_len);
        self.capture_start_pos = pos;
        mem::replace(&mut self.capture, tail)
    }
}

impl<R> Read for ChunkReader<R>
where
    R: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // While the `read` documentation recommends against reading from `buf`,
        // it does not prevent it, and does require callers of `read` to assume
        // we might do this. As consolation, note that we only read back bytes
        // that we know were freshly written, unless of course the source is
        // broken and lies about how many bytes it read.
        let len = self.reader.read(buf)?;
        self.capture.extend_from_slice(&buf[..len]);
        Ok(len)
    }
}

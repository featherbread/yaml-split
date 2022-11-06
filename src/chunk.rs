use std::ffi::c_void;
use std::io::{self, Read};
use std::mem::{self, MaybeUninit};
use std::ops::Deref;

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
    fn new(reader: R) -> Self {
        let parser = {
            let mut parser = Box::new(MaybeUninit::<yaml_parser_t>::uninit());
            if unsafe { yaml_parser_initialize(parser.as_mut_ptr()) }.fail {
                panic!("failed to initialize libyaml parser");
            }
            Box::into_raw(parser).cast::<yaml_parser_t>()
        };

        let read_state = Box::into_raw(Box::new(ReadState {
            reader: ChunkReader::new(reader),
            error: None,
        }));
        unsafe {
            yaml_parser_set_encoding(parser, YAML_UTF8_ENCODING);
            yaml_parser_set_input(parser, Self::read_callback, read_state as *mut c_void);
        }

        Self { parser, read_state }
    }

    unsafe fn read_callback(
        reader: *mut c_void,
        buffer: *mut u8,
        size: u64,
        size_read: *mut u64,
    ) -> i32 {
        const READ_FAILURE: i32 = 0;
        const READ_SUCCESS: i32 = 1;

        let read_state = reader.cast::<ReadState<R>>();
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
            let event = match unsafe { Event::from_parser(self.parser) } {
                Ok(event) => event,
                Err(()) => unsafe {
                    if (*self.parser).error == YAML_READER_ERROR {
                        if let Some(err) = (*self.read_state).error.take() {
                            return Some(Err(err));
                        }
                    }
                    return Some(Err(io::ErrorKind::Other.into()));
                },
            };

            match event.type_ {
                YAML_STREAM_END_EVENT => return None,
                YAML_DOCUMENT_START_EVENT => {
                    let pos = event.start_mark.index;
                    unsafe { (*self.read_state).reader.trim_to_start(pos) };
                }
                YAML_DOCUMENT_END_EVENT => {
                    let pos = event.end_mark.index;
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

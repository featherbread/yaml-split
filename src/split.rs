use std::ffi::{c_void, CStr};
use std::io::Read;
use std::mem::MaybeUninit;

use unsafe_libyaml::*;

pub struct Splitter<R>
where
    R: Read,
{
    parser: *mut yaml_parser_t,
    reader: *mut R,
}

impl<R> Splitter<R>
where
    R: Read,
{
    pub fn new(reader: R) -> Self {
        // SAFETY: We assume that libyaml is correct. To prevent leaks, we do
        // not panic after turning the Box into a raw pointer.
        let parser = unsafe {
            let mut parser = Box::new(MaybeUninit::<yaml_parser_t>::uninit());
            if yaml_parser_initialize(parser.as_mut_ptr()).fail {
                panic!("failed to initialize YAML parser");
            }
            Box::into_raw(parser) as *mut yaml_parser_t
        };

        let reader = Box::into_raw(Box::new(reader));
        // SAFETY: We assume that libyaml is correct.
        unsafe {
            yaml_parser_set_input(parser, Self::read_callback, reader as *mut c_void);
            yaml_parser_set_encoding(parser, YAML_UTF8_ENCODING);
        }

        Self { parser, reader }
    }

    fn read_callback(reader: *mut c_void, buffer: *mut u8, size: u64, size_read: *mut u64) -> i32 {
        // See `yaml_parser_set_input`.
        const FAIL: i32 = 0;
        const SUCCESS: i32 = 1;

        // SAFETY: We assume that libyaml gives us a valid buffer.
        let buf = unsafe { std::slice::from_raw_parts_mut(buffer, size as usize) };

        // SAFETY: After Splitter takes ownership of the provided reader, this
        // is the only place where it is ever used prior to being dropped, so we
        // do not expect any aliasing.
        let len = match unsafe { (*reader.cast::<R>()).read(buf) } {
            Ok(len) => len,
            Err(_) => return FAIL,
        };

        // SAFETY: We assume that libyaml gives us a valid place to write this.
        unsafe { *size_read = len as u64 };
        SUCCESS
    }
}

impl<R> Iterator for Splitter<R>
where
    R: Read,
{
    type Item = (u32, u64);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mut event = unsafe {
                let parser = &mut *self.parser;
                let mut event: MaybeUninit<yaml_event_t> = MaybeUninit::uninit();
                if yaml_parser_parse(parser, event.as_mut_ptr()).fail {
                    panic!(
                        "parser error (code = {}, offset = {}): {}",
                        parser.error as u32,
                        parser.problem_offset,
                        CStr::from_ptr(parser.problem).to_string_lossy(),
                    );
                }
                event.assume_init()
            };

            let result = match event.type_ {
                YAML_STREAM_END_EVENT => None,
                YAML_DOCUMENT_START_EVENT => Some((event.type_ as u32, event.start_mark.index)),
                YAML_DOCUMENT_END_EVENT => Some((event.type_ as u32, event.end_mark.index)),
                _ => {
                    unsafe { yaml_event_delete(&mut event) };
                    continue;
                }
            };
            unsafe { yaml_event_delete(&mut event) };
            return result;
        }
    }
}

impl<R> Drop for Splitter<R>
where
    R: Read,
{
    fn drop(&mut self) {
        unsafe {
            yaml_parser_delete(self.parser);
            drop(Box::from_raw(self.parser));
            drop(Box::from_raw(self.reader));
        }
    }
}

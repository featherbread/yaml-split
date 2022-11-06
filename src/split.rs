use std::ffi::{c_void, CStr};
use std::io::Read;
use std::mem::MaybeUninit;
use std::pin::Pin;

use unsafe_libyaml::*;

pub struct Splitter<R>
where
    R: Read,
{
    parser: Pin<Box<yaml_parser_t>>,
    reader: Pin<Box<R>>,
}

impl<R> Splitter<R>
where
    R: Read,
{
    pub fn new(reader: R) -> Self {
        let reader = Box::pin(reader);
        let parser = {
            let mut parser_uninit = Box::pin(MaybeUninit::<yaml_parser_t>::zeroed());

            // SAFETY: We assume that libyaml works correctly. From what I
            // understand, the only possible failure mode here is a memory
            // allocation error, which we can't reasonably handle with any
            // grace. serde_yaml panics here too, for whatever it's worth.
            if unsafe { yaml_parser_initialize(parser_uninit.as_mut_ptr()).fail } {
                panic!("failed to initialize YAML parser");
            }

            // SAFETY: MaybeUninit<T> is guaranteed by the standard library to
            // have the same size, alignment, and ABI as T. This is roughly
            // similar to the "Initializing an array element-by-element" example
            // in the MaybeUninit docs.
            unsafe { std::mem::transmute::<Pin<Box<MaybeUninit<_>>>, Pin<Box<_>>>(parser_uninit) }
        };

        let mut splitter = Self { parser, reader };

        // SAFETY: Well… the program doesn't seem to crash, nor does Miri seem
        // to blow up on it. This is really ugly, though. I desperately need to
        // get a handle on serde_yaml's whole `Owned` thing.
        unsafe {
            yaml_parser_set_input(
                splitter.parser.as_mut().get_unchecked_mut(),
                Self::read_callback,
                splitter.reader.as_mut().get_unchecked_mut() as *mut R as *mut c_void,
            );
            yaml_parser_set_encoding(
                splitter.parser.as_mut().get_unchecked_mut(),
                YAML_UTF8_ENCODING,
            );
        }

        splitter
    }

    fn read_callback(r: *mut c_void, buffer: *mut u8, size: u64, size_read: *mut u64) -> i32 {
        // SAFETY: Once we take ownership of the reader during construction,
        // this is the only function that ever constructs a &mut to it before it
        // is dropped.
        let r = unsafe { &mut *(r as *mut R) };

        // SAFETY: We assume that libyaml gives us a valid buffer.
        let buf = unsafe { std::slice::from_raw_parts_mut(buffer, size as usize) };

        let len = match r.read(buf) {
            Ok(len) => len,
            Err(_) => return 0,
        };

        // SAFETY: We assume that libyaml gives us a valid place to write this.
        unsafe { *size_read = len as u64 };
        1
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
                let mut event: MaybeUninit<yaml_event_t> = MaybeUninit::uninit();
                let result =
                    yaml_parser_parse(self.parser.as_mut().get_unchecked_mut(), event.as_mut_ptr());
                if result.fail {
                    let problem_str = CStr::from_ptr(self.parser.problem).to_str().unwrap();
                    panic!(
                        "something bad happened ({}): {} @ {}",
                        self.parser.error as u32, problem_str, self.parser.problem_offset,
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
        unsafe { yaml_parser_delete(self.parser.as_mut().get_unchecked_mut()) }
    }
}

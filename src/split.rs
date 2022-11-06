use std::ffi::{c_void, CStr};
use std::io::Read;
use std::mem::MaybeUninit;
use std::pin::Pin;

use unsafe_libyaml::*; // It's called a proof-of-concept, okay?

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

        let mut splitter = Self { parser, reader };

        // I haven't even attempted to run this and I'm already having
        // nightmares about it.
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

    unsafe fn read_callback(
        r: *mut c_void,
        buffer: *mut u8,
        size: u64,
        size_read: *mut u64,
    ) -> i32 {
        // This can't possibly work, right?
        let r = &mut *(r as *mut R);
        let buf = std::slice::from_raw_parts_mut(buffer, size as usize);
        let len = match r.read(buf) {
            Ok(len) => len,
            Err(_) => return 0,
        };
        *size_read = len as u64;
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
                _ => continue,
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

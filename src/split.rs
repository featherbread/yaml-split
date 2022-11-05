use std::mem::MaybeUninit;
use std::pin::Pin;

use unsafe_libyaml::*; // It's called a proof-of-concept, okay?

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

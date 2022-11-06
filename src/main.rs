#[allow(dead_code)]
mod buffer;
#[allow(dead_code)]
mod encode;
mod split;

use std::io::{self, Read};

use encode::{Endianness, UTF32Decoder, UTF8Encoder};
use split::Splitter;

use crate::encode::UTF16Decoder;

static HELLO_UTF32BE: &[u8] = include_bytes!("hello-utf32be.txt");
static HELLO_UTF16LE: &[u8] = include_bytes!("hello-utf16le.txt");

fn main() {
    Splitter::new();
    println!("Splitter didn't crash!");

    print_string(UTF8Encoder::new(UTF16Decoder::new(
        HELLO_UTF16LE,
        Endianness::Little,
    )));
    print_string(UTF8Encoder::new(UTF32Decoder::new(
        HELLO_UTF32BE,
        Endianness::Big,
    )));

    print_bytes_and_string(UTF8Encoder::new(UTF16Decoder::new(
        HELLO_UTF16LE,
        Endianness::Little,
    )));
    print_bytes_and_string(UTF8Encoder::new(UTF32Decoder::new(
        HELLO_UTF32BE,
        Endianness::Big,
    )));
}

fn print_string(r: impl Read) {
    print!("{}", io::read_to_string(r).unwrap());
}

fn print_bytes_and_string(r: impl Read) {
    print!(
        "{}",
        String::from_utf8(
            r.bytes()
                .map(|b| b.unwrap())
                .inspect(|b| print!("{:?} ", b))
                .collect(),
        )
        .unwrap()
    );
}

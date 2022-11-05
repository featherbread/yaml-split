#[allow(dead_code)]
mod buffer;
#[allow(dead_code)]
mod encode;
mod split;

use std::io::{self, Read};

use encode::{Endianness, UTF32Decoder, UTF8Encoder};
use split::Splitter;

static HELLO_UTF32BE: &[u8] = include_bytes!("hello.txt");

fn main() {
    Splitter::new();
    println!("Splitter didn't crash!");

    let encoder = UTF8Encoder::new(UTF32Decoder::new(HELLO_UTF32BE, Endianness::BE));
    print!("{}", io::read_to_string(encoder).unwrap());

    let encoder = UTF8Encoder::new(UTF32Decoder::new(HELLO_UTF32BE, Endianness::BE));
    print!(
        "{}",
        String::from_utf8(
            encoder
                .bytes()
                .map(|b| b.unwrap())
                .inspect(|b| print!("{:?} ", b))
                .collect(),
        )
        .unwrap()
    );
}

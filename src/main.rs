#[allow(dead_code)]
mod buffer;
#[allow(dead_code)]
mod encode;
mod split;

use std::io::Read;

use encode::{Endianness, UTF32Converter};
use split::Splitter;

static HELLO_UTF32BE: &[u8] = include_bytes!("hello.txt");

fn main() {
    Splitter::new();
    println!("Splitter didn't crash!");

    let hello_bytes = HELLO_UTF32BE;
    let encoder = UTF32Converter::new(hello_bytes, Endianness::BE);
    let result = encoder
        .bytes()
        .map(|b| b.unwrap())
        .inspect(|b| print!("{:?} ", b))
        .collect();
    let result_str = String::from_utf8(result).unwrap();
    print!("{result_str}");
}

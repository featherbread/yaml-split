mod encode;
mod split;

use std::io::{self, Read};

use encode::Encoding;
use split::Splitter;

static HELLO_UTF32BE: &[u8] = include_bytes!("hello-utf32be.txt");
static HELLO_UTF16LE: &[u8] = include_bytes!("hello-utf16le.txt");

fn main() {
    {
        Splitter::new();
    }
    println!("Splitter didn't crash!");

    print_string(Encoding::UTF16LE.utf8_reader(HELLO_UTF16LE));
    print_string(Encoding::UTF32BE.utf8_reader(HELLO_UTF32BE));

    print_bytes_and_string(Encoding::UTF16LE.utf8_reader(HELLO_UTF16LE));
    print_bytes_and_string(Encoding::UTF32BE.utf8_reader(HELLO_UTF32BE));
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

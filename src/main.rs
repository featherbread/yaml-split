use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;

use clap::Parser;

mod chunk;
mod encode;

use chunk::Chunker;
use encode::{Encoding, Endianness};

fn main() {
    let cli = Cli::parse();

    let input: Box<dyn BufRead> = match cli.inputfile {
        None => Box::new(io::stdin().lock()),
        Some(filename) => Box::new(BufReader::new(File::open(filename).unwrap())),
    };

    println!("encoding: {:?}", cli.from_code);
    let reader: Box<dyn BufRead> = match cli.from_code {
        None => input,
        Some(code) => Box::new(BufReader::new(code.utf8_reader(input))),
    };

    for chunk in Chunker::new(reader) {
        match chunk {
            Err(err) => panic!("chunker error: {}", err),
            Ok(chunk) => println!(
                ">>> START CHUNK ({} bytes) >>>|{}|<<< END CHUNK <<<",
                chunk.len(),
                chunk,
            ),
        }
    }
}

#[derive(Parser)]
struct Cli {
    #[clap(
        short,
        long,
        name = "encoding",
        help = "Specifies the encoding of the input",
        parse(try_from_str = try_parse_encoding),
    )]
    from_code: Option<Encoding>,

    #[clap(
        name = "inputfile",
        help = "A file to read from instead of standard input"
    )]
    inputfile: Option<PathBuf>,
}

/// Parses a text encoding name in a **ridiculously** loose manner.
fn try_parse_encoding(arg: &str) -> Result<Encoding, String> {
    let arg = arg.to_lowercase();
    let endianness = match &arg {
        arg if arg.contains('b') => Endianness::Big,
        arg if arg.contains('l') => Endianness::Little,
        _ => return Err(format!("can't determine endianness of '{arg}'")),
    };
    Ok(match &arg {
        arg if arg.contains(['1', '6']) => match endianness {
            Endianness::Big => Encoding::UTF16BE,
            Endianness::Little => Encoding::UTF16LE,
        },
        arg if arg.contains(['3', '2']) => match endianness {
            Endianness::Big => Encoding::UTF32BE,
            Endianness::Little => Encoding::UTF32LE,
        },
        _ => return Err(format!("can't determine bitness of '{arg}'")),
    })
}

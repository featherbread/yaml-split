use std::fs::File;
use std::io::{self, BufReader, BufWriter};
use std::path::PathBuf;

use clap::Parser;

mod encode;
#[allow(dead_code)]
mod split;

use encode::{Encoding, Endianness};

fn main() {
    let cli = Cli::parse();
    let mut w = BufWriter::new(io::stdout());
    let mut r = match cli.inputfile {
        None => cli.from_code.utf8_reader(io::stdin().lock()),
        Some(filename) => cli
            .from_code
            .utf8_reader(BufReader::new(File::open(filename).unwrap())),
    };
    if let Err(err) = io::copy(&mut r, &mut w) {
        if err.kind() != io::ErrorKind::BrokenPipe {
            panic!("{}", err);
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
    from_code: Encoding,

    #[clap(
        name = "inputfile",
        help = "A file to read from instead of standard input"
    )]
    inputfile: Option<PathBuf>,
}

fn try_parse_encoding(arg: &str) -> Result<Encoding, String> {
    let arg = arg.to_lowercase();
    let endianness = match &arg {
        arg if arg.contains("be") => Endianness::Big,
        arg if arg.contains("le") => Endianness::Little,
        _ => return Err(format!("can't determine endianness of '{arg}'")),
    };
    Ok(match &arg {
        arg if arg.contains("16") => match endianness {
            Endianness::Big => Encoding::UTF16BE,
            Endianness::Little => Encoding::UTF16LE,
        },
        arg if arg.contains("32") => match endianness {
            Endianness::Big => Encoding::UTF32BE,
            Endianness::Little => Encoding::UTF32LE,
        },
        _ => return Err(format!("can't determine bitness of '{arg}'")),
    })
}

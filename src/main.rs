#![deny(unsafe_op_in_unsafe_fn)]

use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;

use clap::Parser;

#[allow(dead_code)]
mod chunker;
#[allow(dead_code)]
mod encoding;
mod pipecheck;

use chunker::Chunker;
use encoding::Encoder;

fn main() {
	let cli = Cli::parse();
	let input: Box<dyn BufRead> = match cli.inputfile {
		None => Box::new(io::stdin().lock()),
		Some(filename) => Box::new(BufReader::new(File::open(filename).unwrap())),
	};
	let mut output = pipecheck::Writer::new(io::stdout().lock());
	for result in Chunker::new(Encoder::from_reader(input).unwrap()) {
		match result {
			Err(err) => panic!("chunker error: {}", err),
			Ok(doc) => {
				let doc = doc.content();
				writeln!(
					&mut output,
					">>> START CHUNK ({len} bytes) >>>|{doc}|<<< END CHUNK <<<",
					len = doc.len(),
				)
				.unwrap();
			}
		}
	}
}

#[derive(Parser)]
struct Cli {
	#[clap(
		name = "inputfile",
		help = "A file to read from instead of standard input"
	)]
	inputfile: Option<PathBuf>,
}

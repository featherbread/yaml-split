#![deny(unsafe_op_in_unsafe_fn)]

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;

use clap::Parser;

#[allow(dead_code)]
mod chunker;
#[allow(dead_code)]
mod encoding;

use chunker::Chunker;
use encoding::Encoder;

fn main() {
	// SAFETY: libc is assumed to be correct.
	#[cfg(all(unix, not(miri)))]
	unsafe {
		libc::signal(libc::SIGPIPE, libc::SIG_DFL);
	}

	let cli = Cli::parse();
	let input: Box<dyn BufRead> = match cli.inputfile {
		None => Box::new(io::stdin().lock()),
		Some(filename) => Box::new(BufReader::new(File::open(filename).unwrap())),
	};
	for result in Chunker::new(Encoder::from_reader(input).unwrap()) {
		match result {
			Err(err) => panic!("chunker error: {}", err),
			Ok(doc) => println!(
				">>> START CHUNK ({} bytes) >>>|{}|<<< END CHUNK <<<",
				doc.len(),
				&*doc,
			),
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

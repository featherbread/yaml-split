#![deny(unsafe_op_in_unsafe_fn)]

use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{ExitCode, Termination};

use clap::Parser;

#[allow(dead_code)]
mod chunker;
#[allow(dead_code)]
mod encoding;
mod pipecheck;

use chunker::Chunker;
use encoding::Encoder;

fn main() -> Result<(), CleanExit> {
	let cli = Cli::parse();
	let input: Box<dyn BufRead> = match cli.inputfile {
		None => Box::new(io::stdin().lock()),
		Some(filename) => Box::new(BufReader::new(File::open(filename)?)),
	};
	let mut output = pipecheck::Writer::new(io::stdout().lock());
	for result in Chunker::new(Encoder::from_reader(input).unwrap()) {
		let result = result?;
		let doc = result.content();
		writeln!(
			&mut output,
			">>> START CHUNK ({len} bytes) >>>|{doc}|<<< END CHUNK <<<",
			len = doc.len(),
		)?;
	}
	Ok(())
}

#[derive(Parser)]
struct Cli {
	#[clap(
		name = "inputfile",
		help = "A file to read from instead of standard input"
	)]
	inputfile: Option<PathBuf>,
}

struct CleanExit(io::Error);

impl From<io::Error> for CleanExit {
	fn from(value: io::Error) -> Self {
		Self(value)
	}
}

impl Termination for CleanExit {
	fn report(self) -> ExitCode {
		ExitCode::FAILURE
	}
}

impl Debug for CleanExit {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

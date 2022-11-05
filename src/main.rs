#[allow(dead_code)]
#[allow(clippy::all)]
mod split;

use split::Splitter;

fn main() {
    Splitter::new();
    println!("It didn't crash!");
}

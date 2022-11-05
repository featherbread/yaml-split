#[allow(dead_code)]
mod buffer;
mod split;

use split::Splitter;

fn main() {
    Splitter::new();
    println!("It didn't crash!");
}

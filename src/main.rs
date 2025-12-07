use std::path::PathBuf;

use sermmde::pmx::Pmx;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();

    let path = PathBuf::from(args[1].clone());

    let pmx = Pmx::open(&path).unwrap();

    dbg!(pmx);
}

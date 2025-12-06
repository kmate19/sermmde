use std::path::PathBuf;

use sermmde::pmd::Pmd;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();

    let path = PathBuf::from(args[1].clone());

    let pmd = Pmd::open(&path).unwrap();
    dbg!(pmd);
}

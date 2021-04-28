use fwalker::Walker;
use std::env;

fn main() {
    let walker = Walker::from(path_arg()).unwrap();
    for f in walker {
        println!("{:?}", f);
    }
}

fn path_arg() -> String {
    let default = String::from(".");
    env::args().skip(1).next().unwrap_or(default)
}

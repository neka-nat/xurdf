use xurdf::*;

pub fn main() {
    let s = parse_xacro_from_file("data/sample.xacro").unwrap();
    println!("{:?}", s);
}

use xurdf::*;

pub fn main() {
    let urdf = parse_from_file("data/test_robot.urdf");
    println!("{:#?}", urdf);
}

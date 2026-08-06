# xurdf

Parse URDF and Xacro.
This crate does not depend on ROS runtime packages.

## Getting started

```rust
use xurdf::*;

pub fn main() {
    let urdf = parse_urdf_from_file("data/test_robot.urdf");
    println!("{:#?}", urdf);
}
```

`Joint::limit` is `None` when the URDF has no `<limit>` element. When the
element is present, omitted numeric attributes, including `effort` and
`velocity`, default to `0.0`.

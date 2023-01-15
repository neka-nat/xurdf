# xurdf

Parse URDF and Xacro.

## Getting started

```rust
use xurdf::*;

pub fn main() {
    let urdf = parse_urdf_from_file("data/test_robot.urdf");
    println!("{:#?}", urdf);
}
```

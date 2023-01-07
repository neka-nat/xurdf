# xurdf

Parse URDF and Xacro.

## Core features

* Parse URDF and Xacro
* Written by Rust
* Python bindings

## Getting started

```rust
use xurdf::*;

pub fn main() {
    let urdf = parse_urdf_from_file("data/test_robot.urdf");
    println!("{:#?}", urdf);
}
```

You can also use python binding.

```py
import xurdfpy

robot = xurdfpy.parse_urdf_file("data/test_robot.urdf")
print(robot)
```

## Supported Xacro tags

- [x] property
- [ ] property block
- [x] macro
- [ ] include
- [ ] if/unless
- [ ] rospack command
- [ ] Yaml
- [ ] element/attribute

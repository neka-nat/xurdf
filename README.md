# xurdf
[![PyPI version](https://badge.fury.io/py/xurdfpy.svg)](https://badge.fury.io/py/xurdfpy)

Parse URDF and Xacro.
This library does not depend on ROS runtime packages.

## Core features

* Parse URDF and Xacro
* ROS-independent parser library
* Written by Rust
* Python bindings

# Installation

```sh
pip install xurdfpy
```

## Getting started

Rust example is [here](xurdf/README.md).

You can also use python binding.

```py
import xurdfpy

robot = xurdfpy.parse_urdf_file("data/test_robot.urdf")
print(robot)
```

To parse a Xacro file, use `parse_xacro_file`. It returns the expanded XML as a
string.

```py
import xurdfpy

xml = xurdfpy.parse_xacro_file("data/sample.xacro")
print(xml)
```

When resolving `$(find package_name)` or `$(find-pkg-share package_name)`, pass
explicit package paths as a Python dict.

```py
import xurdfpy

xml = xurdfpy.parse_xacro_file_with_package_paths(
    "path/to/robot.urdf.xacro",
    {"my_robot_description": "path/to/my_robot_description"},
)
print(xml)
```

## Supported Xacro tags

- [x] property (`scope=local|parent|global`)
- [x] property block
- [x] macro
- [x] include
- [x] insert_block
- [x] if/unless
- [x] substitution args (`env`, `optenv`, `arg`, `find` via resolver/options)
- [x] package lookup (`find`, `find-pkg-share`, package.xml/env/options)
- [x] Yaml subset (`xacro.load_yaml`, map/list access, `!degrees`/`!radians`)
- [ ] element/attribute

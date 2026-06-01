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
explicit package paths with `package_paths`.

```py
import xurdfpy

xml = xurdfpy.parse_xacro_file(
    "path/to/robot.urdf.xacro",
    package_paths={"my_robot_description": "path/to/my_robot_description"},
)
print(xml)
```

Pass Xacro arguments with `args`.

```py
import xurdfpy

xml = xurdfpy.parse_xacro_file(
    "path/to/robot.urdf.xacro",
    package_paths={"my_robot_description": "path/to/my_robot_description"},
    args={"name": "ur5", "prefix": "left"},
)
```

## Command line

After installation, or directly through `uvx`, convert Xacro to expanded XML with
`xurdf-xacro`.

```sh
uvx --from xurdfpy xurdf-xacro data/sample.xacro
uvx --from xurdfpy xurdf-xacro data/sample.xacro -o robot.urdf
```

Pass package paths for `$(find package_name)` or `$(find-pkg-share package_name)`
with `--package-path`. The option can be repeated.

```sh
uvx --from xurdfpy xurdf-xacro path/to/robot.urdf.xacro \
  --package-path my_robot_description=path/to/my_robot_description \
  -o robot.urdf
```

Xacro arguments are accepted after the input file with `name:=value`.

```sh
uvx --from xurdfpy xurdf-xacro path/to/robot.urdf.xacro prefix:=left
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

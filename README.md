# xurdf
[![PyPI version](https://badge.fury.io/py/xurdfpy.svg)](https://badge.fury.io/py/xurdfpy)

Parse URDF and Xacro.

## Core features

* Parse URDF and Xacro
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

## Supported Xacro tags

- [x] property (`scope=local|parent|global`)
- [x] property block
- [x] macro
- [x] include
- [x] insert_block
- [x] if/unless
- [x] substitution args (`env`, `optenv`, `arg`, `find` via resolver/options)
- [ ] rospack command
- [x] Yaml subset (`xacro.load_yaml`, map/list access, `!degrees`/`!radians`)
- [ ] element/attribute

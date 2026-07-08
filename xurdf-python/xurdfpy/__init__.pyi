from typing import List, Mapping, Optional, Sequence, Tuple, Union

Vector3 = Tuple[float, float, float]
Vector4 = Tuple[float, float, float, float]
Matrix3 = Tuple[
    float, float, float,
    float, float, float,
    float, float, float,
]

class Pose:
    xyz: Vector3
    rpy: Vector3
    def __init__(self, xyz: Sequence[float] = ..., rpy: Sequence[float] = ...) -> None: ...
    def __repr__(self) -> str: ...

class Inertial:
    origin: Pose
    mass: float
    inertia: Matrix3
    def __init__(
        self,
        origin: Pose = ...,
        mass: float = ...,
        inertia: Sequence[float] = ...,
    ) -> None: ...
    def __repr__(self) -> str: ...

class Box:
    size: Vector3
    def __init__(self, size: Sequence[float] = ...) -> None: ...
    def __repr__(self) -> str: ...

class Cylinder:
    radius: float
    length: float
    def __init__(self, radius: float = ..., length: float = ...) -> None: ...
    def __repr__(self) -> str: ...

class Sphere:
    radius: float
    def __init__(self, radius: float = ...) -> None: ...
    def __repr__(self) -> str: ...

class Mesh:
    filename: str
    scale: Optional[Vector3]
    def __init__(self, filename: str = ..., scale: Optional[Sequence[float]] = ...) -> None: ...
    def __repr__(self) -> str: ...

class Material:
    name: Optional[str]
    color: Optional[Vector4]
    def __init__(self, name: Optional[str] = ..., color: Optional[Sequence[float]] = ...) -> None: ...
    def __repr__(self) -> str: ...

Geometry = Union[Box, Cylinder, Sphere, Mesh]

class Visual:
    name: Optional[str]
    origin: Pose
    geometry: Geometry
    material: Optional[Material]
    def __repr__(self) -> str: ...

class Collision:
    name: Optional[str]
    origin: Pose
    geometry: Geometry
    def __repr__(self) -> str: ...

class Link:
    name: str
    inertial: Inertial
    visuals: List[Visual]
    collisions: List[Collision]
    def __repr__(self) -> str: ...

class JointLimit:
    lower: float
    upper: float
    effort: float
    velocity: float
    def __repr__(self) -> str: ...

class Joint:
    name: str
    joint_type: str
    origin: Pose
    parent: str
    child: str
    axis: Vector3
    limit: JointLimit
    def __repr__(self) -> str: ...

class Robot:
    name: str
    materials: List[Material]
    links: List[Link]
    joints: List[Joint]
    def __repr__(self) -> str: ...

def parse_urdf_file(filename: str) -> Robot: ...
def parse_urdf_string(contents: str) -> Robot: ...
def parse_xacro_file(
    filename: str,
    package_paths: Optional[Mapping[str, str]] = ...,
    args: Optional[Mapping[str, str]] = ...,
) -> str: ...
def parse_xacro_string(
    contents: str,
    package_paths: Optional[Mapping[str, str]] = ...,
    args: Optional[Mapping[str, str]] = ...,
) -> str: ...
def xacro_main() -> int: ...

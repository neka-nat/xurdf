use pyo3::prelude::*;
use pyo3::IntoPy;
use xurdf;

#[pyclass]
#[derive(Clone, Debug)]
struct Pose {
    #[pyo3(get, set)]
    xyz: [f64; 3],
    #[pyo3(get, set)]
    rpy: [f64; 3],
}

#[pymethods]
impl Pose {
    #[new]
    #[args(xyz = "[0.0, 0.0, 0.0]", rpy = "[0.0, 0.0, 0.0]")]
    fn new(xyz: [f64; 3], rpy: [f64; 3]) -> Self {
        Pose { xyz, rpy }
    }
    fn __repr__(&self) -> String {
        format!("Pose(xyz: {:?}, rpy: {:?})", self.xyz, self.rpy)
    }
}

#[pyclass]
#[derive(Clone, Debug)]
struct Inertial {
    #[pyo3(get, set)]
    origin: Pose,
    #[pyo3(get, set)]
    mass: f64,
    #[pyo3(get, set)]
    inertia: [f64; 9],
}

#[pymethods]
impl Inertial {
    #[new]
    #[args(
        origin = "Pose::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0])",
        mass = "1.0",
        inertia = "[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]"
    )]
    fn new(origin: Pose, mass: f64, inertia: [f64; 9]) -> Self {
        Inertial {
            origin,
            mass,
            inertia,
        }
    }
    fn __repr__(&self) -> String {
        format!(
            "Inertial(origin: {:?}, mass: {:?}, inertia: {:?})",
            self.origin, self.mass, self.inertia
        )
    }
}

#[pyclass]
#[derive(Clone, Debug)]
struct Box {
    #[pyo3(get, set)]
    size: [f64; 3],
}

#[pymethods]
impl Box {
    #[new]
    #[args(size = "[1.0, 1.0, 1.0]")]
    fn new(size: [f64; 3]) -> Self {
        Box { size }
    }
    fn __repr__(&self) -> String {
        format!("Box(size: {:?})", self.size)
    }
}

#[pyclass]
#[derive(Clone, Debug)]
struct Cylinder {
    #[pyo3(get, set)]
    radius: f64,
    #[pyo3(get, set)]
    length: f64,
}

#[pymethods]
impl Cylinder {
    #[new]
    #[args(radius = "1.0", length = "1.0")]
    fn new(radius: f64, length: f64) -> Self {
        Cylinder { radius, length }
    }
    fn __repr__(&self) -> String {
        format!(
            "Cylinder(radius: {:?}, length: {:?})",
            self.radius, self.length
        )
    }
}

#[pyclass]
#[derive(Clone, Debug)]
struct Sphere {
    #[pyo3(get, set)]
    radius: f64,
}

#[pymethods]
impl Sphere {
    #[new]
    #[args(radius = "1.0")]
    fn new(radius: f64) -> Self {
        Sphere { radius }
    }
    fn __repr__(&self) -> String {
        format!("Sphere(radius: {:?})", self.radius)
    }
}

#[pyclass]
#[derive(Clone, Debug)]
struct Mesh {
    #[pyo3(get, set)]
    filename: String,
    #[pyo3(get, set)]
    scale: Option<[f64; 3]>,
}

#[pymethods]
impl Mesh {
    #[new]
    #[args(filename = "\"\"", scale = "None")]
    fn new(filename: &str, scale: Option<[f64; 3]>) -> Self {
        Mesh {
            filename: filename.to_owned(),
            scale,
        }
    }
    fn __repr__(&self) -> String {
        format!(
            "Mesh(filename: {:?}, scale: {:?})",
            self.filename, self.scale
        )
    }
}

#[derive(Clone, Debug)]
enum Geometry {
    Box(Box),
    Cylinder(Cylinder),
    Sphere(Sphere),
    Mesh(Mesh),
}

impl IntoPy<PyObject> for Geometry {
    fn into_py(self, py: Python) -> PyObject {
        match self {
            Geometry::Box(b) => b.into_py(py),
            Geometry::Cylinder(c) => c.into_py(py),
            Geometry::Sphere(s) => s.into_py(py),
            Geometry::Mesh(m) => m.into_py(py),
        }
    }
}

#[pyclass]
#[derive(Clone, Debug)]
struct Visual {
    #[pyo3(get, set)]
    name: Option<String>,
    #[pyo3(get, set)]
    origin: Pose,
    #[pyo3(get)]
    geometry: Geometry,
}

#[pymethods]
impl Visual {
    fn __repr__(&self) -> String {
        format!(
            "Visual(name: {:?}, origin: {:?}, geometry: {:?})",
            self.name, self.origin, self.geometry
        )
    }
}

#[pyclass]
#[derive(Clone, Debug)]
struct Collision {
    #[pyo3(get, set)]
    name: Option<String>,
    #[pyo3(get, set)]
    origin: Pose,
    #[pyo3(get)]
    geometry: Geometry,
}

#[pymethods]
impl Collision {
    fn __repr__(&self) -> String {
        format!(
            "Collision(name: {:?}, origin: {:?}, geometry: {:?})",
            self.name, self.origin, self.geometry
        )
    }
}

#[pyclass]
#[derive(Clone, Debug)]
struct Link {
    #[pyo3(get, set)]
    name: String,
    #[pyo3(get, set)]
    inertial: Inertial,
    #[pyo3(get, set)]
    visuals: Vec<Visual>,
    #[pyo3(get, set)]
    collisions: Vec<Collision>,
}

#[pymethods]
impl Link {
    fn __repr__(&self) -> String {
        format!(
            "Link(name: {:?}, inertial: {:?}, visuals: {:?}, collisions: {:?})",
            self.name, self.inertial, self.visuals, self.collisions
        )
    }
}

#[pyclass]
#[derive(Clone, Debug)]
struct JointLimit {
    #[pyo3(get, set)]
    lower: f64,
    #[pyo3(get, set)]
    upper: f64,
    #[pyo3(get, set)]
    effort: f64,
    #[pyo3(get, set)]
    velocity: f64,
}

#[pymethods]
impl JointLimit {
    fn __repr__(&self) -> String {
        format!(
            "JointLimit(lower: {:?}, upper: {:?}, effort: {:?}, velocity: {:?})",
            self.lower, self.upper, self.effort, self.velocity
        )
    }
}

#[pyclass]
#[derive(Clone, Debug)]
struct Joint {
    #[pyo3(get, set)]
    name: String,
    #[pyo3(get, set)]
    joint_type: String,
    #[pyo3(get, set)]
    origin: Pose,
    #[pyo3(get, set)]
    parent: String,
    #[pyo3(get, set)]
    child: String,
    #[pyo3(get, set)]
    axis: [f64; 3],
    #[pyo3(get, set)]
    limit: JointLimit,
}

#[pymethods]
impl Joint {
    fn __repr__(&self) -> String {
        format!("Joint(name: {:?}, joint_type: {:?}, origin: {:?}, parent: {:?}, child: {:?}, axis: {:?}, limit: {:?})",
                self.name, self.joint_type, self.origin, self.parent, self.child, self.axis, self.limit)
    }
}

#[pyclass]
#[derive(Clone, Debug)]
struct Robot {
    #[pyo3(get, set)]
    name: String,
    #[pyo3(get, set)]
    links: Vec<Link>,
    #[pyo3(get, set)]
    joints: Vec<Joint>,
}

#[pymethods]
impl Robot {
    fn __repr__(&self) -> String {
        format!(
            "Robot(name: {:?}, links: {:?}, joints: {:?})",
            self.name, self.links, self.joints
        )
    }
}

fn convert_robot(robot: xurdf::Robot) -> Robot {
    let links = robot
        .links
        .iter()
        .map(|link| {
            let inertial = Inertial {
                origin: Pose {
                    xyz: link.inertial.origin.xyz.into(),
                    rpy: link.inertial.origin.rpy.into(),
                },
                mass: link.inertial.mass,
                inertia: [
                    link.inertial.inertia[(0, 0)],
                    link.inertial.inertia[(0, 1)],
                    link.inertial.inertia[(0, 2)],
                    link.inertial.inertia[(1, 0)],
                    link.inertial.inertia[(1, 1)],
                    link.inertial.inertia[(1, 2)],
                    link.inertial.inertia[(2, 0)],
                    link.inertial.inertia[(2, 1)],
                    link.inertial.inertia[(2, 2)],
                ],
            };
            let visuals = link
                .visuals
                .iter()
                .map(|visual| {
                    let geometry = match &visual.geometry {
                        xurdf::Geometry::Box { size } => Geometry::Box(Box {
                            size: [size[0], size[1], size[2]],
                        }),
                        xurdf::Geometry::Cylinder { radius, length } => {
                            Geometry::Cylinder(Cylinder {
                                radius: *radius,
                                length: *length,
                            })
                        }
                        xurdf::Geometry::Sphere { radius } => {
                            Geometry::Sphere(Sphere { radius: *radius })
                        }
                        xurdf::Geometry::Mesh { filename, scale } => Geometry::Mesh(Mesh {
                            filename: filename.clone(),
                            scale: scale.map(|x| x.into()),
                        }),
                    };
                    Visual {
                        name: visual.name.clone(),
                        origin: Pose {
                            xyz: visual.origin.xyz.into(),
                            rpy: visual.origin.rpy.into(),
                        },
                        geometry,
                    }
                })
                .collect();
            let collisions = link
                .collisions
                .iter()
                .map(|collision| {
                    let geometry = match &collision.geometry {
                        xurdf::Geometry::Box { size } => Geometry::Box(Box {
                            size: [size[0], size[1], size[2]],
                        }),
                        xurdf::Geometry::Cylinder { radius, length } => {
                            Geometry::Cylinder(Cylinder {
                                radius: *radius,
                                length: *length,
                            })
                        }
                        xurdf::Geometry::Sphere { radius } => {
                            Geometry::Sphere(Sphere { radius: *radius })
                        }
                        xurdf::Geometry::Mesh { filename, scale } => Geometry::Mesh(Mesh {
                            filename: filename.clone(),
                            scale: scale.map(|x| x.into()),
                        }),
                    };
                    Collision {
                        name: collision.name.clone(),
                        origin: Pose {
                            xyz: collision.origin.xyz.into(),
                            rpy: collision.origin.rpy.into(),
                        },
                        geometry,
                    }
                })
                .collect();
            Link {
                name: link.name.clone(),
                inertial,
                visuals,
                collisions,
            }
        })
        .collect();
    let joints = robot
        .joints
        .iter()
        .map(|joint| Joint {
            name: joint.name.clone(),
            joint_type: joint.joint_type.clone(),
            origin: Pose {
                xyz: joint.origin.xyz.into(),
                rpy: joint.origin.rpy.into(),
            },
            parent: joint.parent.clone(),
            child: joint.child.clone(),
            axis: joint.axis.into(),
            limit: JointLimit {
                lower: joint.limit.lower,
                upper: joint.limit.upper,
                effort: joint.limit.effort,
                velocity: joint.limit.velocity,
            },
        })
        .collect();
    Robot {
        name: robot.name,
        links: links,
        joints: joints,
    }
}

#[pyfunction]
fn parse_urdf_file(filename: &str) -> PyResult<Robot> {
    let robot = xurdf::parse_urdf_from_file(filename)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyException, _>(format!("{}", e)))?;
    Ok(convert_robot(robot))
}

#[pyfunction]
fn parse_urdf_string(contents: &str) -> PyResult<Robot> {
    let robot = xurdf::parse_urdf_from_string(&contents.to_owned())
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyException, _>(format!("{}", e)))?;
    Ok(convert_robot(robot))
}

#[pyfunction]
fn parse_xacro_file(filename: &str) -> PyResult<String> {
    let xacro = xurdf::parse_xacro_from_file(filename)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyException, _>(format!("{}", e)))?;
    Ok(xacro)
}

#[pyfunction]
fn parse_xacro_string(contents: &str) -> PyResult<String> {
    let xacro = xurdf::parse_xacro_from_string(&contents.to_owned())
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyException, _>(format!("{}", e)))?;
    Ok(xacro)
}

#[pymodule]
fn xurdfpy(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Pose>()?;
    m.add_class::<Inertial>()?;
    m.add_class::<Box>()?;
    m.add_class::<Sphere>()?;
    m.add_class::<Cylinder>()?;
    m.add_class::<Mesh>()?;
    m.add_class::<Collision>()?;
    m.add_class::<Visual>()?;
    m.add_class::<Link>()?;
    m.add_class::<Joint>()?;
    m.add_class::<JointLimit>()?;
    m.add_class::<Robot>()?;
    m.add_function(wrap_pyfunction!(parse_urdf_file, m)?)?;
    m.add_function(wrap_pyfunction!(parse_urdf_string, m)?)?;
    m.add_function(wrap_pyfunction!(parse_xacro_file, m)?)?;
    m.add_function(wrap_pyfunction!(parse_xacro_string, m)?)?;
    Ok(())
}

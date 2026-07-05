use pyo3::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use xurdf;

#[pyclass(from_py_object)]
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
    #[pyo3(signature = (xyz = [0.0, 0.0, 0.0], rpy = [0.0, 0.0, 0.0]))]
    fn new(xyz: [f64; 3], rpy: [f64; 3]) -> Self {
        Pose { xyz, rpy }
    }
    fn __repr__(&self) -> String {
        format!("Pose(xyz: {:?}, rpy: {:?})", self.xyz, self.rpy)
    }
}

#[pyclass(from_py_object)]
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
    #[pyo3(signature = (
        origin = Pose::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
        mass = 1.0,
        inertia = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
    ))]
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

#[pyclass(from_py_object)]
#[derive(Clone, Debug)]
struct Box {
    #[pyo3(get, set)]
    size: [f64; 3],
}

#[pymethods]
impl Box {
    #[new]
    #[pyo3(signature = (size = [1.0, 1.0, 1.0]))]
    fn new(size: [f64; 3]) -> Self {
        Box { size }
    }
    fn __repr__(&self) -> String {
        format!("Box(size: {:?})", self.size)
    }
}

#[pyclass(from_py_object)]
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
    #[pyo3(signature = (radius = 1.0, length = 1.0))]
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

#[pyclass(from_py_object)]
#[derive(Clone, Debug)]
struct Sphere {
    #[pyo3(get, set)]
    radius: f64,
}

#[pymethods]
impl Sphere {
    #[new]
    #[pyo3(signature = (radius = 1.0))]
    fn new(radius: f64) -> Self {
        Sphere { radius }
    }
    fn __repr__(&self) -> String {
        format!("Sphere(radius: {:?})", self.radius)
    }
}

#[pyclass(from_py_object)]
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
    #[pyo3(signature = (filename = "", scale = None))]
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

#[pyclass(from_py_object)]
#[derive(Clone, Debug)]
struct Material {
    #[pyo3(get, set)]
    name: Option<String>,
    #[pyo3(get, set)]
    color: Option<[f64; 4]>,
}

#[pymethods]
impl Material {
    #[new]
    #[pyo3(signature = (name = None, color = None))]
    fn new(name: Option<String>, color: Option<[f64; 4]>) -> Self {
        Material { name, color }
    }
    fn __repr__(&self) -> String {
        format!("Material(name: {:?}, color: {:?})", self.name, self.color)
    }
}

#[derive(Clone, Debug)]
enum Geometry {
    Box(Box),
    Cylinder(Cylinder),
    Sphere(Sphere),
    Mesh(Mesh),
}

impl<'py> IntoPyObject<'py> for Geometry {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        match self {
            Geometry::Box(value) => Bound::new(py, value).map(Bound::into_any),
            Geometry::Cylinder(value) => Bound::new(py, value).map(Bound::into_any),
            Geometry::Sphere(value) => Bound::new(py, value).map(Bound::into_any),
            Geometry::Mesh(value) => Bound::new(py, value).map(Bound::into_any),
        }
    }
}

#[pyclass(from_py_object)]
#[derive(Clone, Debug)]
struct Visual {
    #[pyo3(get, set)]
    name: Option<String>,
    #[pyo3(get, set)]
    origin: Pose,
    #[pyo3(get)]
    geometry: Geometry,
    #[pyo3(get, set)]
    material: Option<Material>,
}

#[pymethods]
impl Visual {
    fn __repr__(&self) -> String {
        format!(
            "Visual(name: {:?}, origin: {:?}, geometry: {:?}, material: {:?})",
            self.name, self.origin, self.geometry, self.material
        )
    }
}

#[pyclass(from_py_object)]
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

#[pyclass(from_py_object)]
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

#[pyclass(from_py_object)]
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

#[pyclass(from_py_object)]
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

#[pyclass(from_py_object)]
#[derive(Clone, Debug)]
struct Robot {
    #[pyo3(get, set)]
    name: String,
    #[pyo3(get, set)]
    materials: Vec<Material>,
    #[pyo3(get, set)]
    links: Vec<Link>,
    #[pyo3(get, set)]
    joints: Vec<Joint>,
}

#[pymethods]
impl Robot {
    fn __repr__(&self) -> String {
        format!(
            "Robot(name: {:?}, materials: {:?}, links: {:?}, joints: {:?})",
            self.name, self.materials, self.links, self.joints
        )
    }
}

fn convert_material(material: &xurdf::Material) -> Material {
    Material {
        name: material.name.clone(),
        color: material
            .color
            .as_ref()
            .map(|color| [color[0], color[1], color[2], color[3]]),
    }
}

fn convert_robot(robot: xurdf::Robot) -> Robot {
    let materials = robot.materials.iter().map(convert_material).collect();
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
                        material: visual.material.as_ref().map(convert_material),
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
        materials: materials,
        links: links,
        joints: joints,
    }
}

fn py_exception(err: impl std::fmt::Display) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyException, _>(format!("{:#}", err))
}

#[pyfunction]
fn parse_urdf_file(filename: &str) -> PyResult<Robot> {
    let robot = xurdf::parse_urdf_from_file(filename).map_err(py_exception)?;
    Ok(convert_robot(robot))
}

#[pyfunction]
fn parse_urdf_string(contents: &str) -> PyResult<Robot> {
    let robot = xurdf::parse_urdf_from_string(&contents.to_owned()).map_err(py_exception)?;
    Ok(convert_robot(robot))
}

fn xacro_options(
    package_paths: HashMap<String, String>,
    args: HashMap<String, String>,
) -> xurdf::XacroOptions {
    let options = package_paths.into_iter().fold(
        xurdf::XacroOptions::default(),
        |options, (package, path)| options.with_package_path(package, PathBuf::from(path)),
    );
    args.into_iter().fold(options, |options, (name, value)| {
        options.with_arg(name, value)
    })
}

#[pyfunction]
#[pyo3(signature = (filename, package_paths = None, args = None))]
fn parse_xacro_file(
    filename: &str,
    package_paths: Option<HashMap<String, String>>,
    args: Option<HashMap<String, String>>,
) -> PyResult<String> {
    let xacro = xurdf::parse_xacro_from_file_with_options(
        filename,
        xacro_options(package_paths.unwrap_or_default(), args.unwrap_or_default()),
    )
    .map_err(py_exception)?;
    Ok(xacro)
}

#[pyfunction]
#[pyo3(signature = (contents, package_paths = None, args = None))]
fn parse_xacro_string(
    contents: &str,
    package_paths: Option<HashMap<String, String>>,
    args: Option<HashMap<String, String>>,
) -> PyResult<String> {
    let xacro = xurdf::parse_xacro_from_string_with_options(
        &contents.to_owned(),
        xacro_options(package_paths.unwrap_or_default(), args.unwrap_or_default()),
    )
    .map_err(py_exception)?;
    Ok(xacro)
}

const XACRO_CLI_USAGE: &str = r#"Usage: xurdf-xacro [OPTIONS] <INPUT> [name:=value ...]

Expand a Xacro file and write the expanded XML.

Options:
  -o, --output <PATH>             Write expanded XML to PATH instead of stdout
  -p, --package-path <NAME=PATH>  Resolve $(find NAME) and $(find-pkg-share NAME) to PATH; repeatable
  -h, --help                      Show this help
"#;

#[derive(Debug)]
struct XacroCliArgs {
    input: PathBuf,
    output: Option<PathBuf>,
    package_paths: HashMap<String, PathBuf>,
    args: HashMap<String, String>,
}

fn parse_package_path(spec: &str) -> Result<(String, PathBuf), String> {
    let (package, path) = spec
        .split_once("=")
        .ok_or_else(|| format!("package path must use NAME=PATH: {}", spec))?;
    if package.is_empty() || path.is_empty() {
        return Err(format!("package path must use NAME=PATH: {}", spec));
    }
    Ok((package.to_string(), PathBuf::from(path)))
}

fn parse_xacro_arg(spec: &str) -> Result<(String, String), String> {
    let (name, value) = spec
        .split_once(":=")
        .ok_or_else(|| format!("xacro argument must use NAME:=VALUE: {}", spec))?;
    if name.is_empty() {
        return Err(format!("xacro argument must use NAME:=VALUE: {}", spec));
    }
    Ok((name.to_string(), value.to_string()))
}

fn parse_xacro_cli_args<I, S>(args: I) -> Result<Option<XacroCliArgs>, String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut iter = args.into_iter().map(Into::into).peekable();
    let mut input = None;
    let mut output = None;
    let mut package_paths = HashMap::new();
    let mut xacro_args = HashMap::new();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(None),
            "-o" | "--output" => {
                let path = iter
                    .next()
                    .ok_or_else(|| format!("{} requires a path", arg))?;
                output = Some(PathBuf::from(path));
            }
            "-p" | "--package-path" => {
                let spec = iter
                    .next()
                    .ok_or_else(|| format!("{} requires NAME=PATH", arg))?;
                let (package, path) = parse_package_path(&spec)?;
                package_paths.insert(package, path);
            }
            "--" => {
                for value in iter.by_ref() {
                    if input.is_none() {
                        input = Some(PathBuf::from(value));
                    } else {
                        let (name, value) = parse_xacro_arg(&value)?;
                        xacro_args.insert(name, value);
                    }
                }
                break;
            }
            _ if arg.starts_with("--output=") => {
                let path = arg.trim_start_matches("--output=");
                if path.is_empty() {
                    return Err("--output requires a path".to_string());
                }
                output = Some(PathBuf::from(path));
            }
            _ if arg.starts_with("--package-path=") => {
                let spec = arg.trim_start_matches("--package-path=");
                let (package, path) = parse_package_path(spec)?;
                package_paths.insert(package, path);
            }
            _ if arg.starts_with("-") => return Err(format!("unknown option: {}", arg)),
            _ if input.is_none() => input = Some(PathBuf::from(arg)),
            _ => {
                let (name, value) = parse_xacro_arg(&arg)?;
                xacro_args.insert(name, value);
            }
        }
    }

    let input = input.ok_or_else(|| "missing input xacro file".to_string())?;
    Ok(Some(XacroCliArgs {
        input,
        output,
        package_paths,
        args: xacro_args,
    }))
}

fn xacro_options_for_cli(cli: &XacroCliArgs) -> xurdf::XacroOptions {
    let options = cli.package_paths.iter().fold(
        xurdf::XacroOptions::default(),
        |options, (package, path)| options.with_package_path(package, path),
    );
    cli.args.iter().fold(options, |options, (name, value)| {
        options.with_arg(name, value)
    })
}

fn run_xacro_cli<I, S>(args: I, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let cli = match parse_xacro_cli_args(args) {
        Ok(Some(cli)) => cli,
        Ok(None) => {
            let _ = stdout
                .write_all(XACRO_CLI_USAGE.as_bytes())
                .and_then(|_| stdout.flush());
            return 0;
        }
        Err(err) => {
            let _ = writeln!(stderr, "xurdf-xacro: error: {}", err);
            let _ = writeln!(stderr, "Try `xurdf-xacro --help` for usage.");
            return 2;
        }
    };

    let xml =
        match xurdf::parse_xacro_from_file_with_options(&cli.input, xacro_options_for_cli(&cli)) {
            Ok(xml) => xml,
            Err(err) => {
                let _ = writeln!(stderr, "xurdf-xacro: error: {:#}", err);
                return 1;
            }
        };

    let result = if let Some(output) = &cli.output {
        fs::write(output, xml.as_bytes()).map_err(|err| err.to_string())
    } else {
        stdout
            .write_all(xml.as_bytes())
            .and_then(|_| stdout.flush())
            .map_err(|err| err.to_string())
    };

    if let Err(err) = result {
        let _ = writeln!(stderr, "xurdf-xacro: error: {}", err);
        return 1;
    }

    0
}

#[pyfunction]
fn xacro_main(py: Python<'_>) -> PyResult<i32> {
    let sys = py.import("sys")?;
    let argv: Vec<String> = sys.getattr("argv")?.extract()?;
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();
    Ok(run_xacro_cli(
        argv.into_iter().skip(1),
        &mut stdout,
        &mut stderr,
    ))
}

#[pymodule]
fn xurdfpy(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Pose>()?;
    m.add_class::<Inertial>()?;
    m.add_class::<Box>()?;
    m.add_class::<Sphere>()?;
    m.add_class::<Cylinder>()?;
    m.add_class::<Mesh>()?;
    m.add_class::<Material>()?;
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
    m.add_function(wrap_pyfunction!(xacro_main, m)?)?;
    Ok(())
}

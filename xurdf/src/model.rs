extern crate nalgebra as na;

use na::{Matrix3, Vector3};

#[derive(Debug)]
pub struct Pose {
    pub xyz: Vector3<f64>,
    pub rpy: Vector3<f64>,
}

impl Default for Pose {
    fn default() -> Pose {
        Pose {
            xyz: na::zero(),
            rpy: na::zero(),
        }
    }
}

#[derive(Debug, Default)]
pub struct Inertial {
    pub origin: Pose,
    pub mass: f64,
    pub inertia: Matrix3<f64>,
}

#[derive(Debug)]
pub enum Geometry {
    Box {
        size: Vector3<f64>,
    },
    Cylinder {
        radius: f64,
        length: f64,
    },
    Sphere {
        radius: f64,
    },
    Mesh {
        filename: String,
        scale: Option<Vector3<f64>>,
    },
}

impl Default for Geometry {
    fn default() -> Geometry {
        Geometry::Box { size: na::zero() }
    }
}

#[derive(Debug, Default)]
pub struct Visual {
    pub name: Option<String>,
    pub origin: Pose,
    pub geometry: Geometry,
}

#[derive(Debug, Default)]
pub struct Collision {
    pub name: Option<String>,
    pub origin: Pose,
    pub geometry: Geometry,
}

#[derive(Debug, Default)]
pub struct Link {
    pub name: String,
    pub inertial: Inertial,
    pub visuals: Vec<Visual>,
    pub collisions: Vec<Collision>,
}

#[derive(Debug, Default)]
pub struct JointLimit {
    pub lower: f64,
    pub upper: f64,
    pub effort: f64,
    pub velocity: f64,
}

#[derive(Debug, Default)]
pub struct Joint {
    pub name: String,
    pub joint_type: String,
    pub origin: Pose,
    pub parent: String,
    pub child: String,
    pub axis: Vector3<f64>,
    pub limit: JointLimit,
}

#[derive(Debug, Default)]
pub struct Robot {
    pub name: String,
    pub links: Vec<Link>,
    pub joints: Vec<Joint>,
}

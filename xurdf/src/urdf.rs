extern crate nalgebra as na;

use super::model::*;
use anyhow::Result;
use na::{Matrix3, Vector3, Vector4};
use std::collections::HashMap;
use std::path::Path;

fn parse_string_to_vector3(s: &str) -> Result<Vector3<f64>> {
    let vec = s
        .split(' ')
        .filter_map(|x| x.parse::<f64>().ok())
        .collect::<Vec<_>>();
    if vec.is_empty() {
        Ok(na::zero())
    } else if vec.len() == 3 {
        Ok(Vector3::<f64>::new(vec[0], vec[1], vec[2]))
    } else {
        Err(anyhow::anyhow!(format!(
            "Failed to parse float array in {:?}",
            vec
        )))
    }
}

fn parse_string_to_vector4(s: &str) -> Result<Vector4<f64>> {
    let vec = s
        .split_whitespace()
        .map(|x| x.parse::<f64>())
        .collect::<Result<Vec<_>, _>>()?;
    if vec.len() == 4 {
        Ok(Vector4::<f64>::new(vec[0], vec[1], vec[2], vec[3]))
    } else {
        Err(anyhow::anyhow!(format!(
            "Failed to parse rgba float array in {:?}",
            vec
        )))
    }
}

fn parse_pose(node: roxmltree::Node) -> Result<Pose> {
    let xyz_str = node.attribute("xyz").unwrap_or("");
    let rpy_str = node.attribute("rpy").unwrap_or("");
    Ok(Pose {
        xyz: parse_string_to_vector3(xyz_str)?,
        rpy: parse_string_to_vector3(rpy_str)?,
    })
}

fn parse_inertia(node: roxmltree::Node) -> Result<Matrix3<f64>> {
    let mut inertia = Matrix3::<f64>::identity();
    inertia[(0, 0)] = node
        .attribute("ixx")
        .ok_or(anyhow::anyhow!("Failed to parse inertia ixx"))?
        .parse()?;
    inertia[(0, 1)] = node
        .attribute("ixy")
        .ok_or(anyhow::anyhow!("Failed to parse inertia ixy"))?
        .parse()?;
    inertia[(0, 2)] = node
        .attribute("ixz")
        .ok_or(anyhow::anyhow!("Failed to parse inertia ixz"))?
        .parse()?;
    inertia[(1, 1)] = node
        .attribute("iyy")
        .ok_or(anyhow::anyhow!("Failed to parse inertia iyy"))?
        .parse()?;
    inertia[(1, 2)] = node
        .attribute("iyz")
        .ok_or(anyhow::anyhow!("Failed to parse inertia iyz"))?
        .parse()?;
    inertia[(2, 2)] = node
        .attribute("izz")
        .ok_or(anyhow::anyhow!("Failed to parse inertia izz"))?
        .parse()?;
    inertia[(1, 0)] = inertia[(0, 1)];
    inertia[(2, 0)] = inertia[(0, 2)];
    inertia[(2, 1)] = inertia[(1, 2)];
    Ok(inertia)
}

fn parse_inertial(node: roxmltree::Node) -> Result<Inertial> {
    let mut origin = Pose::default();
    let mut mass = 1.0f64;
    let mut inertia = Matrix3::<f64>::identity();
    for child in node.children() {
        match child.tag_name().name() {
            "origin" => origin = parse_pose(child)?,
            "mass" => {
                mass = child
                    .attribute("value")
                    .ok_or(anyhow::anyhow!("Failed to parse mass value"))?
                    .parse()?
            }
            "inertia" => inertia = parse_inertia(child)?,
            &_ => (),
        }
    }
    Ok(Inertial {
        origin: origin,
        mass: mass,
        inertia: inertia,
    })
}

fn parse_limit(node: roxmltree::Node) -> Result<JointLimit> {
    let lower = node.attribute("lower").unwrap_or("0").parse()?;
    let upper = node.attribute("upper").unwrap_or("0").parse()?;
    let effort = node
        .attribute("effort")
        .ok_or(anyhow::anyhow!("Failed to parse limit effort"))?
        .parse()?;
    let velocity = node
        .attribute("velocity")
        .ok_or(anyhow::anyhow!("Failed to parse limit velocity"))?
        .parse()?;
    Ok(JointLimit {
        lower: lower,
        upper: upper,
        effort: effort,
        velocity: velocity,
    })
}

fn parse_geometry(node: roxmltree::Node) -> Result<Geometry> {
    for child in node.children() {
        match child.tag_name().name() {
            "box" => {
                let size = parse_string_to_vector3(
                    child
                        .attribute("size")
                        .ok_or(anyhow::anyhow!("Failed to parse box size"))?,
                )?;
                return Ok(Geometry::Box { size: size });
            }
            "cylinder" => {
                let radius = child
                    .attribute("radius")
                    .ok_or(anyhow::anyhow!("Failed to parse cylinder radius"))?
                    .parse()?;
                let length = child
                    .attribute("length")
                    .ok_or(anyhow::anyhow!("Failed to parse cylinder length"))?
                    .parse()?;
                return Ok(Geometry::Cylinder {
                    radius: radius,
                    length: length,
                });
            }
            "sphere" => {
                let radius = child
                    .attribute("radius")
                    .ok_or(anyhow::anyhow!("Failed to parse sphere radius"))?
                    .parse()?;
                return Ok(Geometry::Sphere { radius: radius });
            }
            "mesh" => {
                let filename = child
                    .attribute("filename")
                    .ok_or(anyhow::anyhow!("Failed to parse mesh filename"))?;
                let scale = child
                    .attribute("scale")
                    .and_then(|s| parse_string_to_vector3(s).ok());
                return Ok(Geometry::Mesh {
                    filename: filename.to_string(),
                    scale,
                });
            }
            &_ => (),
        }
    }
    Err(anyhow::anyhow!("Failed to parse geometry"))
}

fn parse_material(
    node: roxmltree::Node,
    material_library: &HashMap<String, Material>,
) -> Result<Material> {
    let name = node.attribute("name").map(String::from);
    let mut color = None;
    for child in node.children() {
        match child.tag_name().name() {
            "color" => {
                color =
                    Some(parse_string_to_vector4(child.attribute("rgba").ok_or(
                        anyhow::anyhow!("Failed to parse material color rgba"),
                    )?)?);
            }
            &_ => (),
        }
    }

    if color.is_none() {
        if let Some(name) = name.as_deref() {
            if let Some(material) = material_library.get(name) {
                return Ok(material.clone());
            }
        }
    }

    Ok(Material { name, color })
}

fn parse_materials(node: roxmltree::Node) -> Result<(Vec<Material>, HashMap<String, Material>)> {
    let mut materials = Vec::new();
    let mut material_library = HashMap::new();
    let empty_material_library = HashMap::new();

    for child in node.children() {
        if child.tag_name().name() != "material" {
            continue;
        }
        let material = parse_material(child, &empty_material_library)?;
        if let Some(name) = &material.name {
            material_library.insert(name.clone(), material.clone());
        }
        materials.push(material);
    }

    Ok((materials, material_library))
}

fn parse_visual(
    node: roxmltree::Node,
    material_library: &HashMap<String, Material>,
) -> Result<Visual> {
    let name = node.attribute("name").map(String::from);
    let mut origin = Pose::default();
    let mut geometry = Geometry::Box {
        size: Vector3::<f64>::zeros(),
    };
    let mut material = None;
    for child in node.children() {
        match child.tag_name().name() {
            "origin" => origin = parse_pose(child)?,
            "geometry" => geometry = parse_geometry(child)?,
            "material" => material = Some(parse_material(child, material_library)?),
            &_ => (),
        }
    }
    Ok(Visual {
        name: name,
        origin: origin,
        geometry: geometry,
        material: material,
    })
}

fn parse_collision(node: roxmltree::Node) -> Result<Collision> {
    let name = node.attribute("name").map(String::from);
    let mut origin = Pose::default();
    let mut geometry = Geometry::Box {
        size: Vector3::<f64>::zeros(),
    };
    for child in node.children() {
        match child.tag_name().name() {
            "origin" => origin = parse_pose(child)?,
            "geometry" => geometry = parse_geometry(child)?,
            &_ => (),
        }
    }
    Ok(Collision {
        name: name,
        origin: origin,
        geometry: geometry,
    })
}

fn parse_link(node: roxmltree::Node, material_library: &HashMap<String, Material>) -> Result<Link> {
    let name = String::from(
        node.attribute("name")
            .ok_or(anyhow::anyhow!("Failed to parse link name"))?,
    );
    let mut inertial = Inertial::default();
    let mut visuals: Vec<Visual> = Vec::new();
    let mut collisions: Vec<Collision> = Vec::new();
    for child in node.children() {
        match child.tag_name().name() {
            "inertial" => inertial = parse_inertial(child)?,
            "visual" => visuals.push(parse_visual(child, material_library)?),
            "collision" => collisions.push(parse_collision(child)?),
            &_ => (),
        }
    }
    Ok(Link {
        name: name,
        inertial: inertial,
        visuals: visuals,
        collisions: collisions,
    })
}

fn parse_joint(node: roxmltree::Node) -> Result<Joint> {
    let name = String::from(
        node.attribute("name")
            .ok_or(anyhow::anyhow!("Failed to parse joint name"))?,
    );
    let joint_type = String::from(
        node.attribute("type")
            .ok_or(anyhow::anyhow!("Failed to parse joint type"))?,
    );
    let mut origin = Pose::default();
    let mut jparent = None;
    let mut jchild = None;
    let mut axis = Vector3::new(1.0, 0.0, 0.0);
    let mut limit = JointLimit::default();
    for child in node.children() {
        match child.tag_name().name() {
            "origin" => origin = parse_pose(child)?,
            "parent" => jparent = child.attribute("link"),
            "child" => jchild = child.attribute("link"),
            "axis" => axis = parse_pose(child)?.xyz,
            "limit" => limit = parse_limit(child)?,
            &_ => (),
        }
    }
    Ok(Joint {
        name: name,
        joint_type: joint_type,
        origin: origin,
        parent: String::from(jparent.ok_or(anyhow::anyhow!("Failed to parse joint parent"))?),
        child: String::from(jchild.ok_or(anyhow::anyhow!("Failed to parse joint child"))?),
        axis: axis,
        limit: limit,
    })
}

pub fn parse_urdf_from_string(xml: &str) -> Result<Robot> {
    let doc = roxmltree::Document::parse(xml)?;
    let node = doc.root_element();
    let (materials, material_library) = parse_materials(node)?;
    let links = node
        .children()
        .filter(|n| n.tag_name().name() == "link")
        .map(|n| {
            let link = parse_link(n, &material_library)?;
            Result::<Link>::Ok(link)
        })
        .flatten()
        .collect();
    let joints = node
        .children()
        .filter(|n| n.tag_name().name() == "joint")
        .map(|n| {
            let joint = parse_joint(n)?;
            Result::<Joint>::Ok(joint)
        })
        .flatten()
        .collect();
    Ok(Robot {
        name: String::from(
            node.attribute("name")
                .ok_or(anyhow::anyhow!("Failed to parse robot name"))?,
        ),
        materials: materials,
        links: links,
        joints: joints,
    })
}

pub fn parse_urdf_from_file<P: AsRef<Path>>(path: P) -> Result<Robot> {
    parse_urdf_from_string(&std::fs::read_to_string(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_limit() {
        let limit_without_optional_fields =
            roxmltree::Document::parse(r#"<limit effort="2" velocity="3"/>"#);
        let limit = parse_limit(limit_without_optional_fields.unwrap().root_element());
        assert!(limit.is_ok());
    }

    #[test]
    fn test_parse_joint_default_axis() {
        let robot = parse_urdf_from_string(
            r#"
            <robot name="axis_default">
              <link name="parent"/>
              <link name="child"/>
              <joint name="joint" type="revolute">
                <parent link="parent"/>
                <child link="child"/>
              </joint>
            </robot>
            "#,
        )
        .unwrap();

        assert_eq!(robot.joints[0].axis, Vector3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn test_parse_visual_material_color() {
        let robot = parse_urdf_from_string(
            r#"
            <robot name="materials">
              <material name="blue">
                <color rgba="0 0 .8 1"/>
              </material>
              <link name="base">
                <visual name="from_global">
                  <geometry><box size="1 1 1"/></geometry>
                  <material name="blue"/>
                </visual>
                <visual name="inline">
                  <geometry><box size="1 1 1"/></geometry>
                  <material name="red"><color rgba="1 0 0 .5"/></material>
                </visual>
              </link>
            </robot>
            "#,
        )
        .unwrap();

        assert_eq!(robot.materials.len(), 1);
        assert_eq!(robot.materials[0].name.as_deref(), Some("blue"));
        assert_eq!(
            robot.links[0].visuals[0]
                .material
                .as_ref()
                .and_then(|material| material.color),
            Some(Vector4::new(0.0, 0.0, 0.8, 1.0))
        );
        assert_eq!(
            robot.links[0].visuals[1]
                .material
                .as_ref()
                .and_then(|material| material.color),
            Some(Vector4::new(1.0, 0.0, 0.0, 0.5))
        );
    }
}

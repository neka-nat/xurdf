use super::eval::*;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use xmltree::{Element, XMLNode};

const XACRO_PREFIX: &str = "xacro";

#[derive(Clone, Debug)]
struct Macro {
    params: Vec<String>,
    body: Element,
}

#[derive(Debug)]
struct Context {
    properties: HashMap<String, PropertyValue>,
    macros: HashMap<String, Macro>,
}

impl Context {
    pub fn parse_and_write_xacro(&mut self, elem: &Element) -> Element {
        let mut new_elem = elem.clone();
        new_elem.children.clear();
        match elem.prefix.as_ref() {
            Some(_p) => {}
            None => {
                for (name, val) in elem.attributes.iter() {
                    let new_value = eval_text(val, &self.properties);
                    new_elem.attributes.insert(name.clone(), new_value);
                }
            }
        }
        for child in elem.children.iter() {
            match child.as_element() {
                Some(node) => {
                    let default_ns = &"".to_string();
                    let prefix = node.prefix.as_ref().unwrap_or(default_ns);
                    let tags = (prefix.as_str(), node.name.as_str());
                    match tags {
                        (XACRO_PREFIX, "property") => {
                            let name = node.attributes["name"].clone();
                            let value = node.attributes["value"].clone();
                            self.properties.insert(
                                name,
                                PropertyValue {
                                    raw_value: eval_text(&value, &self.properties),
                                },
                            );
                        }
                        (XACRO_PREFIX, "macro") => {
                            let name = node.attributes["name"].clone();
                            let params = node.attributes["params"].clone();
                            let params = params.split_whitespace().map(|s| s.to_string()).collect();
                            self.macros.insert(
                                name,
                                Macro {
                                    params,
                                    body: node.clone(),
                                },
                            );
                        }
                        (XACRO_PREFIX, name) => {
                            let new_nodes = self.handle_macro_call(&node, name);
                            new_elem.children.extend(new_nodes);
                        }
                        (_, _) => {
                            let new_node = self.parse_and_write_xacro(&node);
                            new_elem.children.push(XMLNode::Element(new_node));
                        }
                    }
                }
                None => {}
            }
        }
        new_elem
    }
    fn handle_macro_call(&self, node: &Element, name: &str) -> Vec<XMLNode> {
        let mut local_context = Context {
            properties: self.properties.clone(),
            macros: self.macros.clone(),
        };
        let this_macro = &self.macros[name];
        node.attributes.iter().for_each(|(k, v)| {
            local_context.properties.insert(
                k.clone(),
                PropertyValue {
                    raw_value: eval_text(v, &self.properties),
                },
            );
        });
        let new_elem = local_context.parse_and_write_xacro(&this_macro.body);
        new_elem.children
    }
}

pub fn parse_xacro_from_string(xml: &String) -> Result<String> {
    let elem = Element::parse(xml.as_bytes())?;
    let mut context = Context {
        properties: HashMap::new(),
        macros: HashMap::new(),
    };
    let new_elem = context.parse_and_write_xacro(&elem);
    let mut w = Vec::new();
    new_elem.write(&mut w)?;
    String::from_utf8(w).map_err(|e| e.into())
}

pub fn parse_xacro_from_file<P: AsRef<Path>>(path: P) -> Result<String> {
    parse_xacro_from_string(&std::fs::read_to_string(path)?).map_err(|e| e.into())
}

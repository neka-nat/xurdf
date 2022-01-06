use std::collections::{HashMap, HashSet};
use std::ops::Index;

use roxmltree;

struct Table {
    data: HashMap<String, String>,
    parent: Box<Table>,
    root: Box<Table>,
    depth: usize,
    unevaluated: HashSet<String>,
    recursive: Vec<String>,
}

impl Table {
    fn resolve<'a>(&'a self, key: &String) -> &'a String {
        &self.data[key]
    }
}

impl Index<String> for Table {
    type Output = String;

    fn index(&self, key: String) -> &Self::Output {
        if self.data.contains_key(&key) {
            &self.resolve(&key)
        } else if self.parent.depth > 0 {
            &self.parent.data[&key]
        } else {
            panic!("Key not found: {}", key);
        }
    }
}

fn eval_all(node: &mut roxmltree::Node) {

}

pub fn parse_xacro_and_rewrite_doc<'a>(doc: &'a mut roxmltree::Document<'a>) -> &'a roxmltree::Document<'a> {
    let mut node = doc.root_element();
    eval_all(&mut node);
    node.document()
}
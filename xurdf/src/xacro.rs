use super::eval::*;
use anyhow::{bail, Context as AnyhowContext, Result};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use xmltree::{Element, XMLNode};

const XACRO_PREFIX: &str = "xacro";
const UNSUPPORTED_XACRO_TAGS: &[&str] = &["element", "attribute"];

#[derive(Clone, Debug)]
struct Macro {
    params: Vec<MacroParam>,
    body: Element,
}

#[derive(Clone, Debug)]
struct MacroParam {
    name: String,
    kind: MacroParamKind,
    default: Option<MacroDefault>,
}

impl MacroParam {
    fn symbol_key(&self) -> String {
        match self.kind {
            MacroParamKind::Value => self.name.clone(),
            MacroParamKind::SingleBlock => format!("*{}", self.name),
            MacroParamKind::ContentBlock => format!("**{}", self.name),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MacroParamKind {
    Value,
    SingleBlock,
    ContentBlock,
}

#[derive(Clone, Debug)]
struct MacroDefault {
    forward: bool,
    value: Option<String>,
}

#[derive(Clone, Debug)]
struct BlockValue {
    nodes: Vec<XMLNode>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PropertyScope {
    Local,
    Parent,
    Global,
}

#[derive(Clone, Debug, Default)]
struct XacroContext {
    properties: HashMap<String, XacroValue>,
    blocks: HashMap<String, BlockValue>,
    macros: HashMap<String, Macro>,
    args: HashMap<String, String>,
}

pub trait XacroSubstitutionResolver: std::fmt::Debug + Send + Sync {
    fn resolve_arg(&self, _name: &str) -> Result<Option<String>> {
        Ok(None)
    }

    fn resolve_find(&self, _package: &str) -> Result<Option<PathBuf>> {
        Ok(None)
    }
}

#[derive(Debug, Default)]
struct EmptyXacroSubstitutionResolver;

impl XacroSubstitutionResolver for EmptyXacroSubstitutionResolver {}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct XacroOptions {
    pub require_macro_params: bool,
    pub args: HashMap<String, String>,
    pub package_paths: HashMap<String, PathBuf>,
    substitution_resolver: Arc<dyn XacroSubstitutionResolver>,
}

impl Default for XacroOptions {
    fn default() -> Self {
        Self {
            require_macro_params: true,
            args: HashMap::new(),
            package_paths: HashMap::new(),
            substitution_resolver: Arc::new(EmptyXacroSubstitutionResolver),
        }
    }
}

impl XacroOptions {
    pub fn with_arg(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.args.insert(name.into(), value.into());
        self
    }

    pub fn with_package_path(
        mut self,
        package: impl Into<String>,
        path: impl Into<PathBuf>,
    ) -> Self {
        self.package_paths.insert(package.into(), path.into());
        self
    }

    pub fn with_substitution_resolver<R>(mut self, resolver: R) -> Self
    where
        R: XacroSubstitutionResolver + 'static,
    {
        self.substitution_resolver = Arc::new(resolver);
        self
    }
}

#[derive(Debug)]
pub struct XacroProcessor {
    context: XacroContext,
    options: XacroOptions,
    include_stack: Vec<PathBuf>,
    parent_property_exports: HashMap<String, XacroValue>,
    global_property_exports: HashMap<String, XacroValue>,
    parent_block_exports: HashMap<String, BlockValue>,
    global_block_exports: HashMap<String, BlockValue>,
}

impl Default for XacroProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl XacroProcessor {
    pub fn new() -> Self {
        Self::with_options(XacroOptions::default())
    }

    pub fn with_options(options: XacroOptions) -> Self {
        Self {
            context: context_from_options(&options),
            options,
            include_stack: Vec::new(),
            parent_property_exports: HashMap::new(),
            global_property_exports: HashMap::new(),
            parent_block_exports: HashMap::new(),
            global_block_exports: HashMap::new(),
        }
    }

    pub fn current_file(&self) -> Option<&Path> {
        self.include_stack.last().map(PathBuf::as_path)
    }

    pub fn current_dir(&self) -> Option<&Path> {
        self.current_file().and_then(Path::parent)
    }

    pub fn process_string(&mut self, xml: &str) -> Result<String> {
        self.reset_context();
        self.process_source_string(xml)
    }

    pub fn process_file<P: AsRef<Path>>(&mut self, path: P) -> Result<String> {
        self.reset_context();
        self.process_file_inner(path.as_ref())
    }

    pub fn process_document(&mut self, elem: &Element) -> Result<Element> {
        self.reset_context();
        self.process_element(elem)
    }

    fn reset_context(&mut self) {
        self.context = context_from_options(&self.options);
        self.include_stack.clear();
        self.parent_property_exports.clear();
        self.global_property_exports.clear();
        self.parent_block_exports.clear();
        self.global_block_exports.clear();
    }

    fn process_file_inner(&mut self, path: &Path) -> Result<String> {
        let new_elem = self.process_file_to_element_inner(path)?;
        write_element_to_string(&new_elem)
    }

    fn process_file_to_element_inner(&mut self, path: &Path) -> Result<Element> {
        let canonical_path = std::fs::canonicalize(path)
            .with_context(|| format!("failed to resolve xacro file `{}`", path.display()))?;

        if self.include_stack.iter().any(|p| p == &canonical_path) {
            bail!(
                "cyclic xacro include detected: {}",
                format_include_stack(&self.include_stack, &canonical_path)
            );
        }

        let xml = std::fs::read_to_string(&canonical_path)
            .with_context(|| format!("failed to read xacro file `{}`", canonical_path.display()))?;

        self.include_stack.push(canonical_path.clone());
        let result = self
            .process_source_element(&xml)
            .with_context(|| format!("while processing xacro file `{}`", canonical_path.display()));
        self.include_stack.pop();
        result
    }

    fn process_source_string(&mut self, xml: &str) -> Result<String> {
        let new_elem = self.process_source_element(xml)?;
        write_element_to_string(&new_elem)
    }

    fn process_source_element(&mut self, xml: &str) -> Result<Element> {
        let elem = Element::parse(xml.as_bytes()).context("failed to parse xacro XML")?;
        self.process_element(&elem)
    }

    fn process_element(&mut self, elem: &Element) -> Result<Element> {
        let mut new_elem = elem.clone();
        new_elem.children.clear();

        if elem.prefix.is_none() {
            for (name, val) in elem.attributes.iter() {
                let new_value = self.eval_text(val)?;
                new_elem.attributes.insert(name.clone(), new_value);
            }
        }

        for child in elem.children.iter() {
            let Some(node) = child.as_element() else {
                continue;
            };

            match xacro_tag_name(node) {
                Some("property") => self.handle_property(node)?,
                Some("arg") => self.handle_arg(node)?,
                Some("macro") => self.handle_macro_definition(node)?,
                Some("include") => {
                    let new_nodes = self.handle_include(node)?;
                    new_elem.children.extend(new_nodes);
                }
                Some("insert_block") => {
                    let new_nodes = self.handle_insert_block(node)?;
                    new_elem.children.extend(new_nodes);
                }
                Some("if") => {
                    if self.evaluate_condition(node, "if")? {
                        let new_node = self.process_element(node)?;
                        new_elem.children.extend(new_node.children);
                    }
                }
                Some("unless") => {
                    if !self.evaluate_condition(node, "unless")? {
                        let new_node = self.process_element(node)?;
                        new_elem.children.extend(new_node.children);
                    }
                }
                Some(name) => {
                    let new_nodes = self.handle_macro_call(node, name)?;
                    new_elem.children.extend(new_nodes);
                }
                None => {
                    let new_node = self.process_element(node)?;
                    new_elem.children.push(XMLNode::Element(new_node));
                }
            }
        }

        Ok(new_elem)
    }

    fn handle_property(&mut self, node: &Element) -> Result<()> {
        let name = self.eval_text(required_attr(node, "name")?)?;
        let value = node.attributes.get("value");
        let default = node.attributes.get("default");
        let scope = self.property_scope(node)?;
        let has_element_children = node
            .children
            .iter()
            .any(|child| child.as_element().is_some());

        if value.is_some() && default.is_some() {
            bail!("xacro:property attributes `value` and `default` are mutually exclusive");
        }
        if (value.is_some() || default.is_some()) && has_element_children {
            bail!("xacro:property cannot mix `value`/`default` attributes with block content");
        }

        if let Some(value) = value {
            let value = self.eval_value(value)?;
            self.set_property(name, value, scope);
        } else if let Some(default) = default {
            if !self.context.properties.contains_key(&name) {
                let value = self.eval_value(default)?;
                self.set_property(name, value, scope);
            }
        } else {
            let name = format!("**{}", name);
            let block = BlockValue {
                nodes: element_child_nodes(&node.children),
            };
            self.set_block(name, block, scope);
        }
        Ok(())
    }

    fn handle_arg(&mut self, node: &Element) -> Result<()> {
        let name = required_attr(node, "name")?.to_string();
        let value = node
            .attributes
            .get("default")
            .or_else(|| node.attributes.get("value"))
            .with_context(|| format!("xacro:arg `{}` requires `default` attribute", name))?;

        if !self.context.args.contains_key(&name) {
            let value = self.eval_text(value)?;
            self.context.args.insert(name, value);
        }
        Ok(())
    }

    fn handle_macro_definition(&mut self, node: &Element) -> Result<()> {
        let name = required_attr(node, "name")?.to_string();
        let params = required_attr(node, "params")?;
        self.context.macros.insert(
            name,
            Macro {
                params: parse_macro_args(params)?,
                body: node.clone(),
            },
        );
        Ok(())
    }

    fn handle_include(&mut self, node: &Element) -> Result<Vec<XMLNode>> {
        let filename = self.eval_text(required_attr(node, "filename")?)?;
        if filename.is_empty() {
            bail!("xacro:include requires a non-empty `filename` attribute");
        }

        let include_path = self.resolve_include_path(&filename);
        let included = self
            .process_file_to_element_inner(&include_path)
            .with_context(|| format!("while including xacro file `{}`", filename))?;
        Ok(included.children)
    }

    fn handle_insert_block(&mut self, node: &Element) -> Result<Vec<XMLNode>> {
        let name = self.eval_text(required_attr(node, "name")?)?;
        let block = self
            .context
            .blocks
            .get(&format!("**{}", name))
            .or_else(|| self.context.blocks.get(&format!("*{}", name)))
            .cloned()
            .with_context(|| format!("undefined xacro block `{}`", name))?;
        self.expand_block_nodes(&block.nodes)
    }

    fn evaluate_condition(&self, node: &Element, tag_name: &str) -> Result<bool> {
        self.eval_bool(required_attr(node, "value")?)
            .with_context(|| format!("failed to evaluate xacro:{} condition", tag_name))
    }

    fn handle_macro_call(&mut self, node: &Element, name: &str) -> Result<Vec<XMLNode>> {
        let Some(this_macro) = self.context.macros.get(name).cloned() else {
            if UNSUPPORTED_XACRO_TAGS.contains(&name) {
                bail!("unsupported xacro tag `xacro:{}`", name);
            }
            bail!("undefined xacro macro `xacro:{}`", name);
        };

        let value_param_names = this_macro
            .params
            .iter()
            .filter(|param| param.kind == MacroParamKind::Value)
            .map(|param| param.name.as_str())
            .collect::<HashSet<_>>();
        for attr_name in node.attributes.keys() {
            if !value_param_names.contains(attr_name.as_str()) {
                bail!(
                    "invalid parameter `{}` for macro `xacro:{}`",
                    attr_name,
                    name
                );
            }
        }

        let mut local_processor = XacroProcessor {
            context: self.context.clone(),
            options: self.options.clone(),
            include_stack: self.include_stack.clone(),
            parent_property_exports: HashMap::new(),
            global_property_exports: HashMap::new(),
            parent_block_exports: HashMap::new(),
            global_block_exports: HashMap::new(),
        };

        for param in this_macro
            .params
            .iter()
            .filter(|param| param.kind == MacroParamKind::Value)
        {
            if let Some(attr_value) = node.attributes.get(&param.name) {
                local_processor
                    .context
                    .properties
                    .insert(param.name.clone(), self.eval_value(attr_value)?);
            } else if let Some(default) = &param.default {
                local_processor
                    .context
                    .properties
                    .insert(param.name.clone(), self.eval_macro_default(param, default)?);
            } else if self.options.require_macro_params {
                bail!(
                    "missing required parameter `{}` for macro `xacro:{}`",
                    param.name,
                    name
                );
            }
        }

        let mut child_blocks = node.children.iter().filter_map(XMLNode::as_element);
        for param in this_macro
            .params
            .iter()
            .filter(|param| param.kind != MacroParamKind::Value)
        {
            let Some(block) = child_blocks.next() else {
                bail!(
                    "not enough blocks when instantiating macro `xacro:{}`; missing `{}`",
                    name,
                    param.symbol_key()
                );
            };

            let nodes = match param.kind {
                MacroParamKind::Value => unreachable!(),
                MacroParamKind::SingleBlock => vec![XMLNode::Element(block.clone())],
                MacroParamKind::ContentBlock => element_child_nodes(&block.children),
            };
            local_processor
                .context
                .blocks
                .insert(param.symbol_key(), BlockValue { nodes });
        }

        if let Some(block) = child_blocks.next() {
            bail!(
                "unused block `{}` when instantiating macro `xacro:{}`",
                block.name,
                name
            );
        }

        let new_elem = local_processor.process_element(&this_macro.body)?;
        self.apply_scope_exports(&local_processor);
        Ok(new_elem.children)
    }

    fn property_scope(&self, node: &Element) -> Result<PropertyScope> {
        let Some(scope) = node.attributes.get("scope") else {
            return Ok(PropertyScope::Local);
        };
        match self.eval_text(scope)?.as_str() {
            "local" => Ok(PropertyScope::Local),
            "parent" => Ok(PropertyScope::Parent),
            "global" => Ok(PropertyScope::Global),
            scope => bail!("unsupported xacro:property scope `{}`", scope),
        }
    }

    fn set_property(&mut self, name: String, value: XacroValue, scope: PropertyScope) {
        self.context.properties.insert(name.clone(), value.clone());
        match scope {
            PropertyScope::Local => {}
            PropertyScope::Parent => {
                self.parent_property_exports.insert(name, value);
            }
            PropertyScope::Global => {
                self.global_property_exports.insert(name, value);
            }
        }
    }

    fn set_block(&mut self, name: String, block: BlockValue, scope: PropertyScope) {
        self.context.blocks.insert(name.clone(), block.clone());
        match scope {
            PropertyScope::Local => {}
            PropertyScope::Parent => {
                self.parent_block_exports.insert(name, block);
            }
            PropertyScope::Global => {
                self.global_block_exports.insert(name, block);
            }
        }
    }

    fn apply_scope_exports(&mut self, child: &XacroProcessor) {
        for (name, value) in child.parent_property_exports.iter() {
            self.context.properties.insert(name.clone(), value.clone());
        }
        for (name, block) in child.parent_block_exports.iter() {
            self.context.blocks.insert(name.clone(), block.clone());
        }
        for (name, value) in child.global_property_exports.iter() {
            self.context.properties.insert(name.clone(), value.clone());
            self.global_property_exports
                .insert(name.clone(), value.clone());
        }
        for (name, block) in child.global_block_exports.iter() {
            self.context.blocks.insert(name.clone(), block.clone());
            self.global_block_exports
                .insert(name.clone(), block.clone());
        }
    }

    fn eval_macro_default(&self, param: &MacroParam, default: &MacroDefault) -> Result<XacroValue> {
        if default.forward {
            if let Some(value) = self.context.properties.get(&param.name) {
                return Ok(value.clone());
            }
        }

        if let Some(value) = &default.value {
            let value = self.eval_value(value)?;
            return Ok(match value {
                XacroValue::String(value) => {
                    XacroValue::String(strip_balanced_quotes(&value).to_string())
                }
                value => value,
            });
        }

        bail!("undefined property to forward: {}", param.name)
    }

    fn expand_block_nodes(&mut self, nodes: &[XMLNode]) -> Result<Vec<XMLNode>> {
        let mut expanded = Vec::new();
        for node in nodes {
            match node {
                XMLNode::Element(element) => {
                    expanded.push(XMLNode::Element(self.process_element(element)?));
                }
                other => expanded.push(other.clone()),
            }
        }
        Ok(expanded)
    }

    fn eval_text(&self, text: &str) -> Result<String> {
        try_eval_text_with_values(
            text,
            &self.context.properties,
            &|expr| self.resolve_substitution(expr),
            &|expr| self.resolve_value_expression(expr),
        )
    }

    fn eval_value(&self, text: &str) -> Result<XacroValue> {
        try_eval_value_with_values(
            text,
            &self.context.properties,
            &|expr| self.resolve_substitution(expr),
            &|expr| self.resolve_value_expression(expr),
        )
    }

    fn eval_bool(&self, text: &str) -> Result<bool> {
        try_get_boolean_value_with_values(
            text,
            &self.context.properties,
            &|expr| self.resolve_substitution(expr),
            &|expr| self.resolve_value_expression(expr),
        )
    }

    fn resolve_value_expression(&self, expr: &str) -> Result<Option<XacroValue>> {
        let Some(arg) = xacro_function_arg(expr, "xacro.load_yaml")
            .or_else(|| xacro_function_arg(expr, "load_yaml"))
        else {
            return Ok(None);
        };

        let filename = self.eval_value(&format!("${{{}}}", arg))?.raw_value();
        if filename.is_empty() {
            bail!("xacro.load_yaml requires a non-empty filename");
        }

        let path = self.resolve_include_path(&filename);
        let yaml = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read YAML file `{}`", path.display()))?;
        parse_simple_yaml(&yaml)
            .with_context(|| format!("failed to parse YAML file `{}`", path.display()))
            .map(Some)
    }

    fn resolve_include_path(&self, filename: &str) -> PathBuf {
        let path = Path::new(filename);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.current_dir()
                .unwrap_or_else(|| Path::new("."))
                .join(path)
        }
    }

    fn resolve_substitution(&self, expr: &str) -> Result<String> {
        let mut parts = expr.split_whitespace();
        let Some(command) = parts.next() else {
            bail!("empty xacro substitution command");
        };

        match command {
            "cwd" => Ok(self
                .current_dir()
                .map(Path::to_path_buf)
                .unwrap_or(std::env::current_dir()?)
                .to_string_lossy()
                .into_owned()),
            "env" => {
                let name = required_substitution_arg(command, parts.next())?;
                ensure_no_extra_args(command, &mut parts)?;
                std::env::var(name)
                    .with_context(|| format!("environment variable `{}` is not set", name))
            }
            "optenv" => {
                let name = required_substitution_arg(command, parts.next())?;
                match std::env::var(name) {
                    Ok(value) => Ok(value),
                    Err(_) => Ok(parts.collect::<Vec<_>>().join(" ")),
                }
            }
            "arg" => {
                let name = required_substitution_arg(command, parts.next())?;
                ensure_no_extra_args(command, &mut parts)?;
                self.resolve_arg(name)
            }
            "find" => {
                let package = required_substitution_arg(command, parts.next())?;
                ensure_no_extra_args(command, &mut parts)?;
                self.resolve_find(package)
            }
            _ => bail!("unsupported xacro substitution command `$({})`", expr),
        }
    }

    fn resolve_arg(&self, name: &str) -> Result<String> {
        if let Some(value) = self.context.args.get(name) {
            return Ok(value.clone());
        }

        if let Some(value) = self.options.substitution_resolver.resolve_arg(name)? {
            return Ok(value);
        }

        bail!("undefined xacro argument `{}`", name)
    }

    fn resolve_find(&self, package: &str) -> Result<String> {
        if let Some(path) = self.options.package_paths.get(package) {
            return Ok(path.to_string_lossy().into_owned());
        }

        if let Some(path) = self.options.substitution_resolver.resolve_find(package)? {
            return Ok(path.to_string_lossy().into_owned());
        }

        bail!(
            "unable to resolve package `{}` for `$(find {})`",
            package,
            package
        )
    }
}

fn xacro_function_arg<'a>(expr: &'a str, name: &str) -> Option<&'a str> {
    let expr = expr.trim();
    let rest = expr.strip_prefix(name)?.trim_start();
    if !rest.starts_with('(') || !rest.ends_with(')') {
        return None;
    }
    Some(rest[1..rest.len() - 1].trim())
}

#[derive(Clone, Debug)]
struct YamlLine {
    line_number: usize,
    indent: usize,
    key: String,
    value: Option<String>,
}

fn parse_simple_yaml(input: &str) -> Result<XacroValue> {
    let mut lines = Vec::new();
    for (idx, raw_line) in input.lines().enumerate() {
        let line = strip_yaml_comment(raw_line).trim_end().to_string();
        if line.trim().is_empty() || line.trim_start().starts_with("---") {
            continue;
        }

        let indent = line.chars().take_while(|ch| *ch == ' ').count();
        let trimmed = line.trim_start();
        let (key, value) = split_yaml_key_value(trimmed)
            .with_context(|| format!("invalid YAML mapping on line {}", idx + 1))?;
        lines.push(YamlLine {
            line_number: idx + 1,
            indent,
            key: strip_balanced_quotes(key.trim()).to_string(),
            value: value.map(|value| value.trim().to_string()),
        });
    }

    if lines.is_empty() {
        return Ok(XacroValue::Null);
    }

    let mut index = 0;
    let root_indent = lines[0].indent;
    let value = parse_yaml_map(&lines, &mut index, root_indent)?;
    if index != lines.len() {
        bail!(
            "unexpected YAML content on line {}",
            lines[index].line_number
        );
    }
    Ok(value)
}

fn parse_yaml_map(lines: &[YamlLine], index: &mut usize, indent: usize) -> Result<XacroValue> {
    let mut map = BTreeMap::new();

    while *index < lines.len() {
        let line = &lines[*index];
        if line.indent < indent {
            break;
        }
        if line.indent > indent {
            bail!("unexpected YAML indentation on line {}", line.line_number);
        }

        *index += 1;
        let value = if let Some(value) = &line.value {
            parse_yaml_scalar(value)
                .with_context(|| format!("invalid YAML scalar on line {}", line.line_number))?
        } else if *index < lines.len() && lines[*index].indent > indent {
            parse_yaml_map(lines, index, lines[*index].indent)?
        } else {
            XacroValue::Map(BTreeMap::new())
        };
        map.insert(line.key.clone(), value);
    }

    Ok(XacroValue::Map(map))
}

fn parse_yaml_scalar(value: &str) -> Result<XacroValue> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(XacroValue::String(String::new()));
    }

    if let Some(rest) = value.strip_prefix("!degrees") {
        let degrees = parse_yaml_number(rest.trim())?;
        return Ok(XacroValue::Number(degrees.to_radians()));
    }
    if let Some(rest) = value.strip_prefix("!radians") {
        return Ok(XacroValue::Number(parse_yaml_number(rest.trim())?));
    }
    if value.starts_with('[') && value.ends_with(']') {
        return split_inline_yaml_list(&value[1..value.len() - 1])
            .into_iter()
            .map(|item| parse_yaml_scalar(&item))
            .collect::<Result<Vec<_>>>()
            .map(XacroValue::List);
    }

    let unquoted = strip_balanced_quotes(value);
    if unquoted != value {
        return Ok(XacroValue::String(unquoted.to_string()));
    }

    match value.to_ascii_lowercase().as_str() {
        "true" => Ok(XacroValue::Bool(true)),
        "false" => Ok(XacroValue::Bool(false)),
        "null" | "~" => Ok(XacroValue::Null),
        _ => match value.parse::<f64>() {
            Ok(number) => Ok(XacroValue::Number(number)),
            Err(_) => Ok(XacroValue::String(value.to_string())),
        },
    }
}

fn parse_yaml_number(value: &str) -> Result<f64> {
    let XacroValue::Number(value) = parse_yaml_scalar(value)? else {
        bail!("expected numeric YAML tag argument");
    };
    Ok(value)
}

fn strip_yaml_comment(line: &str) -> String {
    let mut quote = None;
    let mut escape = false;
    for (idx, ch) in line.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if ch == '\\' {
            escape = true;
            continue;
        }
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            '#' => return line[..idx].to_string(),
            _ => {}
        }
    }
    line.to_string()
}

fn split_yaml_key_value(line: &str) -> Option<(&str, Option<&str>)> {
    let mut quote = None;
    let mut escape = false;
    for (idx, ch) in line.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if ch == '\\' {
            escape = true;
            continue;
        }
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            ':' => {
                let key = line[..idx].trim();
                if key.is_empty() {
                    return None;
                }
                let value = line[idx + ch.len_utf8()..].trim();
                return Some((key, (!value.is_empty()).then_some(value)));
            }
            _ => {}
        }
    }
    None
}

fn split_inline_yaml_list(value: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut start = 0usize;
    let mut quote = None;
    let mut escape = false;
    let mut bracket_depth = 0usize;

    for (idx, ch) in value.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if ch == '\\' {
            escape = true;
            continue;
        }
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            ',' if bracket_depth == 0 => {
                let item = value[start..idx].trim();
                if !item.is_empty() {
                    values.push(item.to_string());
                }
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }

    let item = value[start..].trim();
    if !item.is_empty() {
        values.push(item.to_string());
    }
    values
}

fn context_from_options(options: &XacroOptions) -> XacroContext {
    XacroContext {
        args: options.args.clone(),
        ..XacroContext::default()
    }
}

fn xacro_tag_name(node: &Element) -> Option<&str> {
    match node.prefix.as_deref() {
        Some(XACRO_PREFIX) => Some(node.name.as_str()),
        _ => None,
    }
}

fn required_attr<'a>(node: &'a Element, attr: &str) -> Result<&'a str> {
    node.attributes
        .get(attr)
        .map(String::as_str)
        .with_context(|| format!("xacro:{} requires `{}` attribute", node.name, attr))
}

fn required_substitution_arg<'a>(command: &str, value: Option<&'a str>) -> Result<&'a str> {
    value.with_context(|| format!("$({}) requires an argument", command))
}

fn ensure_no_extra_args<'a>(
    command: &str,
    parts: &mut impl Iterator<Item = &'a str>,
) -> Result<()> {
    if let Some(extra) = parts.next() {
        bail!("$({}) received unexpected argument `{}`", command, extra);
    }
    Ok(())
}

fn parse_macro_args(s: &str) -> Result<Vec<MacroParam>> {
    let tokens = tokenize_macro_params(s)?;
    let mut params = Vec::new();
    let mut symbols = HashSet::new();

    for token in tokens {
        let param = parse_macro_param(&token)?;
        let key = param.symbol_key();
        if !symbols.insert(key.clone()) {
            bail!("duplicate macro parameter `{}`", key);
        }
        params.push(param);
    }

    Ok(params)
}

fn parse_macro_param(token: &str) -> Result<MacroParam> {
    let (raw_name, default) = match token.split_once(":=") {
        Some((name, default)) => (name.trim(), Some(parse_macro_default(default.trim()))),
        None => match token.split_once('=') {
            Some((name, default)) => (name.trim(), Some(parse_macro_default(default.trim()))),
            None => (token.trim(), None),
        },
    };

    let (kind, name) = if let Some(name) = raw_name.strip_prefix("**") {
        (MacroParamKind::ContentBlock, name)
    } else if let Some(name) = raw_name.strip_prefix('*') {
        (MacroParamKind::SingleBlock, name)
    } else {
        (MacroParamKind::Value, raw_name)
    };

    if name.is_empty() {
        bail!("macro parameter name cannot be empty");
    }
    if kind != MacroParamKind::Value && default.is_some() {
        bail!(
            "block parameter `{}` cannot define a default value",
            raw_name
        );
    }

    Ok(MacroParam {
        name: name.to_string(),
        kind,
        default,
    })
}

fn parse_macro_default(default: &str) -> MacroDefault {
    if let Some(value) = default.strip_prefix("^|") {
        MacroDefault {
            forward: true,
            value: Some(value.to_string()),
        }
    } else if let Some(value) = default.strip_prefix('^') {
        MacroDefault {
            forward: true,
            value: if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            },
        }
    } else {
        MacroDefault {
            forward: false,
            value: Some(default.to_string()),
        }
    }
}

fn tokenize_macro_params(s: &str) -> Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut chars = s.char_indices().peekable();

    while let Some((start, ch)) = chars.next() {
        if ch.is_whitespace() {
            continue;
        }

        let mut end = start + ch.len_utf8();
        let mut quote = None;
        let mut brace_depth = 0usize;
        let mut paren_depth = 0usize;

        while let Some(&(idx, ch)) = chars.peek() {
            if let Some(q) = quote {
                chars.next();
                end = idx + ch.len_utf8();
                if ch == q {
                    quote = None;
                }
                continue;
            }

            match ch {
                '\'' | '"' => {
                    chars.next();
                    quote = Some(ch);
                    end = idx + ch.len_utf8();
                }
                '{' => {
                    chars.next();
                    brace_depth += 1;
                    end = idx + ch.len_utf8();
                }
                '}' => {
                    chars.next();
                    brace_depth = brace_depth.saturating_sub(1);
                    end = idx + ch.len_utf8();
                }
                '(' => {
                    chars.next();
                    paren_depth += 1;
                    end = idx + ch.len_utf8();
                }
                ')' => {
                    chars.next();
                    paren_depth = paren_depth.saturating_sub(1);
                    end = idx + ch.len_utf8();
                }
                _ if ch.is_whitespace() && brace_depth == 0 && paren_depth == 0 => break,
                _ => {
                    chars.next();
                    end = idx + ch.len_utf8();
                }
            }
        }

        let token = s[start..end].trim();
        if !token.is_empty() {
            tokens.push(token.to_string());
        }
    }

    Ok(tokens)
}

fn element_child_nodes(nodes: &[XMLNode]) -> Vec<XMLNode> {
    nodes
        .iter()
        .filter_map(|node| node.as_element().cloned().map(XMLNode::Element))
        .collect()
}

fn strip_balanced_quotes(value: &str) -> &str {
    if value.len() >= 2
        && ((value.starts_with('\'') && value.ends_with('\''))
            || (value.starts_with('"') && value.ends_with('"')))
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

fn write_element_to_string(elem: &Element) -> Result<String> {
    let mut w = Vec::new();
    elem.write(&mut w)
        .context("failed to serialize expanded xacro XML")?;
    String::from_utf8(w).map_err(|e| e.into())
}

fn format_include_stack(stack: &[PathBuf], repeated: &Path) -> String {
    let mut paths = stack
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    paths.push(repeated.display().to_string());
    paths.join(" -> ")
}

pub fn parse_xacro_from_string(xml: &str) -> Result<String> {
    XacroProcessor::new().process_string(xml)
}

pub fn parse_xacro_from_string_with_options(xml: &str, options: XacroOptions) -> Result<String> {
    XacroProcessor::with_options(options).process_string(xml)
}

pub fn parse_xacro_from_file<P: AsRef<Path>>(path: P) -> Result<String> {
    XacroProcessor::new().process_file(path)
}

pub fn parse_xacro_from_file_with_options<P: AsRef<Path>>(
    path: P,
    options: XacroOptions,
) -> Result<String> {
    XacroProcessor::with_options(options).process_file(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const NS: &str = "http://www.ros.org/wiki/xacro";

    #[test]
    fn expands_macro_with_property_and_condition() {
        let xml = format!(
            r#"<robot xmlns:xacro="{NS}">
  <xacro:property name="side" value="left"/>
  <xacro:macro name="link_macro" params="name suffix:=_link">
    <xacro:if value="${{side == 'left'}}">
      <link name="${{name}}${{suffix}}"/>
    </xacro:if>
  </xacro:macro>
  <xacro:link_macro name="base"/>
</robot>"#
        );

        let result = parse_xacro_from_string(&xml).unwrap();

        assert!(result.contains(r#"<link name="base_link" />"#));
        assert!(!result.contains("xacro:macro"));
    }

    #[test]
    fn expands_single_block_macro_parameter() {
        let xml = format!(
            r#"<robot xmlns:xacro="{NS}">
  <xacro:macro name="default_inertial" params="mass *origin">
    <inertial>
      <xacro:insert_block name="origin"/>
      <mass value="${{mass}}"/>
    </inertial>
  </xacro:macro>
  <xacro:default_inertial mass="1.0">
    <origin xyz="0 0 0" rpy="0 0 0"/>
  </xacro:default_inertial>
</robot>"#
        );

        let result = parse_xacro_from_string(&xml).unwrap();

        assert!(result.contains("<origin "));
        assert!(result.contains(r#"xyz="0 0 0""#));
        assert!(result.contains(r#"rpy="0 0 0""#));
        assert!(result.contains(r#"<mass value="1" />"#));
    }

    #[test]
    fn expands_content_block_macro_parameter() {
        let xml = format!(
            r#"<robot xmlns:xacro="{NS}">
  <xacro:macro name="group" params="**links">
    <wrapper>
      <xacro:insert_block name="links"/>
    </wrapper>
  </xacro:macro>
  <xacro:group>
    <contents>
      <link name="a"/>
      <link name="b"/>
    </contents>
  </xacro:group>
</robot>"#
        );

        let result = parse_xacro_from_string(&xml).unwrap();

        assert!(result.contains(r#"<wrapper><link name="a" /><link name="b" /></wrapper>"#));
        assert!(!result.contains("contents"));
    }

    #[test]
    fn expands_property_block() {
        let xml = format!(
            r#"<robot xmlns:xacro="{NS}">
  <xacro:property name="origin_block">
    <origin xyz="1 2 3"/>
  </xacro:property>
  <joint name="j">
    <xacro:insert_block name="origin_block"/>
  </joint>
</robot>"#
        );

        let result = parse_xacro_from_string(&xml).unwrap();

        assert!(result.contains(r#"<joint name="j"><origin xyz="1 2 3" /></joint>"#));
    }

    #[test]
    fn supports_forwarded_macro_default() {
        let xml = format!(
            r#"<robot xmlns:xacro="{NS}">
  <xacro:property name="prefix" value="outer"/>
  <xacro:macro name="make_link" params="prefix:=^ suffix:=_link">
    <link name="${{prefix}}${{suffix}}"/>
  </xacro:macro>
  <xacro:make_link/>
</robot>"#
        );

        let result = parse_xacro_from_string(&xml).unwrap();

        assert!(result.contains(r#"<link name="outer_link" />"#));
    }

    #[test]
    fn strips_quotes_from_macro_default_literals() {
        let xml = format!(
            r#"<robot xmlns:xacro="{NS}">
  <xacro:macro name="make_link" params="prefix:='' suffix:=_link">
    <link name="${{prefix}}${{suffix}}"/>
  </xacro:macro>
  <xacro:make_link/>
</robot>"#
        );

        let result = parse_xacro_from_string(&xml).unwrap();

        assert!(result.contains(r#"<link name="_link" />"#));
    }

    #[test]
    fn includes_relative_file_and_reuses_its_macro() {
        let dir = temp_fixture_dir("include-relative");
        let include_dir = dir.join("common");
        fs::create_dir_all(&include_dir).unwrap();
        fs::write(
            include_dir.join("links.xacro"),
            format!(
                r#"<robot xmlns:xacro="{NS}">
  <xacro:property name="included_suffix" value="_from_include"/>
  <xacro:macro name="make_link" params="name">
    <link name="${{name}}${{included_suffix}}"/>
  </xacro:macro>
</robot>"#
            ),
        )
        .unwrap();
        let main = dir.join("main.xacro");
        fs::write(
            &main,
            format!(
                r#"<robot xmlns:xacro="{NS}">
  <xacro:include filename="common/links.xacro"/>
  <xacro:make_link name="base"/>
</robot>"#
            ),
        )
        .unwrap();

        let result = parse_xacro_from_file(&main).unwrap();

        assert!(result.contains(r#"<link name="base_from_include" />"#));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn include_filename_can_use_xacro_arg() {
        let dir = temp_fixture_dir("include-arg");
        let include_dir = dir.join("common");
        fs::create_dir_all(&include_dir).unwrap();
        fs::write(
            include_dir.join("links.xacro"),
            format!(
                r#"<robot xmlns:xacro="{NS}"><xacro:macro name="make_link" params="name"><link name="${{name}}"/></xacro:macro></robot>"#
            ),
        )
        .unwrap();
        let main = dir.join("main.xacro");
        fs::write(
            &main,
            format!(
                r#"<robot xmlns:xacro="{NS}">
  <xacro:arg name="include_file" default="common/links.xacro"/>
  <xacro:include filename="$(arg include_file)"/>
  <xacro:make_link name="base"/>
</robot>"#
            ),
        )
        .unwrap();

        let result = parse_xacro_from_file(&main).unwrap();

        assert!(result.contains(r#"<link name="base" />"#));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn substitution_args_support_arg_optenv_and_find() {
        let dir = temp_fixture_dir("substitution-args");
        let missing_env = format!("XURDF_MISSING_OPTENV_{}", std::process::id());
        let xml = format!(
            r#"<robot xmlns:xacro="{NS}">
  <xacro:arg name="prefix" default="default"/>
  <link name="$(arg prefix)_$(optenv {missing_env} fallback)" mesh="$(find fixture_pkg)/meshes/base.stl"/>
</robot>"#
        );
        let options = XacroOptions::default()
            .with_arg("prefix", "provided")
            .with_package_path("fixture_pkg", &dir);

        let result = parse_xacro_from_string_with_options(&xml, options).unwrap();

        assert!(result.contains(r#"name="provided_fallback""#));
        assert!(result.contains(&format!(
            r#"mesh="{}/meshes/base.stl""#,
            dir.to_string_lossy()
        )));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn errors_on_cyclic_include() {
        let dir = temp_fixture_dir("include-cycle");
        let a = dir.join("a.xacro");
        let b = dir.join("b.xacro");
        fs::write(
            &a,
            format!(r#"<robot xmlns:xacro="{NS}"><xacro:include filename="b.xacro"/></robot>"#),
        )
        .unwrap();
        fs::write(
            &b,
            format!(r#"<robot xmlns:xacro="{NS}"><xacro:include filename="a.xacro"/></robot>"#),
        )
        .unwrap();

        let err = parse_xacro_from_file(&a).unwrap_err();
        let err = format!("{:#}", err);

        assert!(err.contains("cyclic xacro include detected"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn errors_on_invalid_macro_parameter() {
        let xml = format!(
            r#"<robot xmlns:xacro="{NS}">
  <xacro:macro name="make_link" params="name"><link name="${{name}}"/></xacro:macro>
  <xacro:make_link name="base" extra="bad"/>
</robot>"#
        );

        let err = parse_xacro_from_string(&xml).unwrap_err().to_string();

        assert!(err.contains("invalid parameter `extra` for macro `xacro:make_link`"));
    }

    #[test]
    fn errors_on_unused_macro_block() {
        let xml = format!(
            r#"<robot xmlns:xacro="{NS}">
  <xacro:macro name="make_link" params="name"><link name="${{name}}"/></xacro:macro>
  <xacro:make_link name="base"><origin/></xacro:make_link>
</robot>"#
        );

        let err = parse_xacro_from_string(&xml).unwrap_err().to_string();

        assert!(err.contains("unused block `origin` when instantiating macro `xacro:make_link`"));
    }

    #[test]
    fn allows_empty_property_block() {
        let xml = format!(r#"<robot xmlns:xacro="{NS}"><xacro:property name="foo"/></robot>"#);

        let result = parse_xacro_from_string(&xml).unwrap();

        assert!(!result.contains("xacro:property"));
    }

    #[test]
    fn errors_on_undefined_macro() {
        let xml = format!(r#"<robot xmlns:xacro="{NS}"><xacro:missing/></robot>"#);

        let err = parse_xacro_from_string(&xml).unwrap_err().to_string();

        assert!(err.contains("undefined xacro macro `xacro:missing`"));
    }

    #[test]
    fn errors_on_missing_required_macro_parameter() {
        let xml = format!(
            r#"<robot xmlns:xacro="{NS}">
  <xacro:macro name="link_macro" params="name"><link name="${{name}}"/></xacro:macro>
  <xacro:link_macro/>
</robot>"#
        );

        let err = parse_xacro_from_string(&xml).unwrap_err().to_string();

        assert!(err.contains("missing required parameter `name` for macro `xacro:link_macro`"));
    }

    #[test]
    fn macro_parent_scope_exports_property_to_caller() {
        let xml = format!(
            r#"<robot xmlns:xacro="{NS}">
  <xacro:macro name="set_values" params="value">
    <xacro:property name="local_only" value="hidden"/>
    <xacro:property name="exported" value="${{value}}" scope="parent"/>
  </xacro:macro>
  <xacro:set_values value="ok"/>
  <link name="${{exported}}" local="${{local_only}}"/>
</robot>"#
        );

        let result = parse_xacro_from_string(&xml).unwrap();

        assert!(result.contains(r#"name="ok""#));
        assert!(result.contains(r#"local="${local_only}""#));
    }

    #[test]
    fn macro_global_scope_exports_through_nested_callers() {
        let xml = format!(
            r#"<robot xmlns:xacro="{NS}">
  <xacro:macro name="inner" params="">
    <xacro:property name="global_name" value="visible" scope="global"/>
  </xacro:macro>
  <xacro:macro name="outer" params="">
    <xacro:inner/>
    <link name="inside_${{global_name}}"/>
  </xacro:macro>
  <xacro:outer/>
  <link name="outside_${{global_name}}"/>
</robot>"#
        );

        let result = parse_xacro_from_string(&xml).unwrap();

        assert!(result.contains(r#"name="inside_visible""#));
        assert!(result.contains(r#"name="outside_visible""#));
    }

    #[test]
    fn errors_on_invalid_property_scope() {
        let xml = format!(
            r#"<robot xmlns:xacro="{NS}">
  <xacro:property name="bad" value="1" scope="sideways"/>
</robot>"#
        );

        let err = parse_xacro_from_string(&xml).unwrap_err().to_string();

        assert!(err.contains("unsupported xacro:property scope `sideways`"));
    }

    #[test]
    fn processor_does_not_leak_context_between_documents() {
        let first = format!(
            r#"<robot xmlns:xacro="{NS}">
  <xacro:macro name="link_macro" params="name"><link name="${{name}}"/></xacro:macro>
  <xacro:link_macro name="first"/>
</robot>"#
        );
        let second =
            format!(r#"<robot xmlns:xacro="{NS}"><xacro:link_macro name="second"/></robot>"#);
        let mut processor = XacroProcessor::new();

        processor.process_string(&first).unwrap();
        let err = processor.process_string(&second).unwrap_err().to_string();

        assert!(err.contains("undefined xacro macro `xacro:link_macro`"));
    }

    #[test]
    fn loads_yaml_and_accesses_nested_values() {
        let dir = temp_fixture_dir("load-yaml");
        fs::write(
            dir.join("config.yaml"),
            r#"
robot:
  name: arm
joints:
  shoulder:
    enabled: true
    max_effort: 150.0
    min_position: !degrees -180
values:
  offsets: [1, 2, foo]
"#,
        )
        .unwrap();
        let main = dir.join("main.xacro");
        fs::write(
            &main,
            format!(
                r#"<robot xmlns:xacro="{NS}">
  <xacro:property name="cfg_path" value="config.yaml"/>
  <xacro:property name="cfg" value="${{xacro.load_yaml(cfg_path)}}"/>
  <xacro:property name="shoulder" value="${{cfg['joints']['shoulder']}}"/>
  <xacro:if value="${{shoulder['enabled']}}">
    <link name="${{cfg.robot.name}}" effort="${{shoulder['max_effort']}}" lower="${{shoulder['min_position']}}" offset="${{cfg['values']['offsets'][2]}}"/>
  </xacro:if>
</robot>"#
            ),
        )
        .unwrap();

        let result = parse_xacro_from_file(&main).unwrap();

        assert!(result.contains(r#"name="arm""#));
        assert!(result.contains(r#"effort="150""#));
        assert!(result.contains(r#"lower="-3.141592653589793""#));
        assert!(result.contains(r#"offset="foo""#));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn yaml_boolean_false_skips_if_body() {
        let dir = temp_fixture_dir("load-yaml-false");
        fs::write(
            dir.join("config.yaml"),
            r#"
feature:
  enabled: false
"#,
        )
        .unwrap();
        let main = dir.join("main.xacro");
        fs::write(
            &main,
            format!(
                r#"<robot xmlns:xacro="{NS}">
  <xacro:property name="cfg" value="${{load_yaml('config.yaml')}}"/>
  <xacro:if value="${{cfg.feature.enabled}}">
    <link name="enabled"/>
  </xacro:if>
</robot>"#
            ),
        )
        .unwrap();

        let result = parse_xacro_from_file(&main).unwrap();

        assert!(!result.contains(r#"name="enabled""#));
        let _ = fs::remove_dir_all(dir);
    }

    fn temp_fixture_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("xurdf-xacro-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}

//! The bounded, strict YAML event parser under the tolerant frontmatter
//! layer. Everything here refuses rather than guesses: aliases, duplicate
//! keys, complex keys, extra documents, and any input over the budgets.

use yaml_rust2::parser::{Event, Parser};
use yaml_rust2::scanner::TScalarStyle;

use super::{MAX_NODES, MAX_YAML_BYTES, Map, Value};

const MAX_DEPTH: usize = 16;

pub fn parse(yaml: &str) -> Result<Map, String> {
    if yaml.len() > MAX_YAML_BYTES {
        return Err(format!(
            "frontmatter is {} bytes — the limit is {MAX_YAML_BYTES}",
            yaml.len()
        ));
    }
    let mut parser = Parser::new_from_str(yaml);
    let mut stack: Vec<Node> = Vec::new();
    let mut root: Option<Value> = None;
    let mut nodes = 0usize;
    loop {
        let (event, _) = parser.next_token().map_err(|e| e.to_string())?;
        nodes += 1;
        if nodes > MAX_NODES {
            return Err(format!("frontmatter exceeds {MAX_NODES} YAML nodes"));
        }
        match event {
            Event::StreamStart | Event::DocumentStart | Event::DocumentEnd | Event::Nothing => {}
            Event::StreamEnd => break,
            Event::Alias(_) => return Err("YAML aliases are not accepted in frontmatter".into()),
            Event::Scalar(text, style, _, _) => {
                place(&mut stack, &mut root, scalar_value(text, style))?;
            }
            Event::SequenceStart(_, _) => push_node(&mut stack, &root, Node::List(Vec::new()))?,
            Event::SequenceEnd => {
                let Some(Node::List(items)) = stack.pop() else {
                    return Err("malformed YAML sequence".into());
                };
                place(&mut stack, &mut root, Value::List(items))?;
            }
            Event::MappingStart(_, _) => push_node(
                &mut stack,
                &root,
                Node::Map {
                    map: Map::default(),
                    pending_key: None,
                },
            )?,
            Event::MappingEnd => {
                let Some(Node::Map { map, .. }) = stack.pop() else {
                    return Err("malformed YAML mapping".into());
                };
                place(&mut stack, &mut root, Value::Map(map))?;
            }
        }
    }
    match root {
        None | Some(Value::Null) => Ok(Map::default()),
        Some(Value::Map(map)) => Ok(map),
        Some(_) => Err("frontmatter is not a key/value map".into()),
    }
}

enum Node {
    Map {
        map: Map,
        pending_key: Option<String>,
    },
    List(Vec<Value>),
}

fn push_node(stack: &mut Vec<Node>, root: &Option<Value>, node: Node) -> Result<(), String> {
    if root.is_some() && stack.is_empty() {
        return Err("multiple YAML documents in frontmatter".into());
    }
    if stack.len() >= MAX_DEPTH {
        return Err(format!("frontmatter nests deeper than {MAX_DEPTH} levels"));
    }
    if let Some(Node::Map {
        pending_key: None, ..
    }) = stack.last()
    {
        return Err("YAML complex keys are not accepted in frontmatter".into());
    }
    stack.push(node);
    Ok(())
}

fn place(stack: &mut [Node], root: &mut Option<Value>, value: Value) -> Result<(), String> {
    match stack.last_mut() {
        None => {
            if root.is_some() {
                return Err("multiple YAML documents in frontmatter".into());
            }
            *root = Some(value);
        }
        Some(Node::List(items)) => items.push(value),
        Some(Node::Map { map, pending_key }) => match pending_key.take() {
            Some(key) => map.insert(key, value)?,
            None => match value {
                Value::Scalar(key) => *pending_key = Some(key),
                Value::Null => *pending_key = Some(String::new()),
                _ => return Err("YAML complex keys are not accepted in frontmatter".into()),
            },
        },
    }
    Ok(())
}

fn scalar_value(text: String, style: TScalarStyle) -> Value {
    if style == TScalarStyle::Plain
        && (text.is_empty() || text == "~" || text.eq_ignore_ascii_case("null"))
    {
        return Value::Null;
    }
    Value::Scalar(text)
}

//! Preserve the Python CLI's YAML 1.1 scalar and merge semantics. yaml-rust2
//! handles syntax; this module resolves the JSON-compatible manifest values.
use anyhow::{Context, Result, anyhow, bail};
use regex::Regex;
use serde_json::{Map, Number, Value};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use yaml_rust2::parser::{Event, MarkedEventReceiver, Parser, Tag};
use yaml_rust2::scanner::{Marker, Scanner, TScalarStyle, Token, TokenType};

#[derive(Default)]
struct Events(Vec<(Event, Marker)>);
impl MarkedEventReceiver for Events {
    fn on_event(&mut self, event: Event, marker: Marker) {
        self.0.push((event, marker));
    }
}

fn events(text: &str) -> Result<Vec<(Event, Marker)>> {
    let mut events = Events::default();
    Parser::new_from_str(text)
        .load(&mut events, true)
        .context("Invalid YAML syntax")?;
    Ok(events.0)
}

pub fn parse(text: &str) -> Result<Value> {
    // The event parser replaces duplicate anchor names with fresh numeric IDs.
    // Check lexical names first to preserve PyYAML's duplicate-anchor error.
    let mut anchors = HashSet::new();
    let mut scanner = Scanner::new(text.chars());
    while let Some(Token(marker, token)) = scanner.next_token().context("Invalid YAML syntax")? {
        if let TokenType::Anchor(name) = token
            && !anchors.insert(name.clone())
        {
            bail!("Duplicate YAML anchor {name} at line {}", marker.line());
        }
    }
    let events = events(text)?;
    if events
        .iter()
        .filter(|(e, _)| *e == Event::DocumentStart)
        .count()
        > 1
    {
        bail!("Expected one YAML document");
    }
    let Some(start) = events.iter().position(|(e, _)| *e == Event::DocumentStart) else {
        return Ok(Value::Null);
    };
    let mut builder = Builder {
        events,
        index: start + 1,
        anchors: HashMap::new(),
        alias_bytes: 0,
    };
    builder.value(0)
}

struct Builder {
    events: Vec<(Event, Marker)>,
    index: usize,
    anchors: HashMap<usize, Value>,
    alias_bytes: usize,
}

impl Builder {
    fn value(&mut self, depth: usize) -> Result<Value> {
        if depth > 128 {
            bail!("YAML nesting is too deep");
        }
        let (event, marker) = self
            .events
            .get(self.index)
            .cloned()
            .ok_or_else(|| anyhow!("Incomplete YAML value"))?;
        self.index += 1;
        let (value, anchor) = match event {
            Event::Scalar(text, style, anchor, tag) => {
                (scalar(&text, style, tag.as_ref())?, anchor)
            }
            Event::Alias(anchor) => {
                let value = self.anchors.get(&anchor).ok_or_else(|| {
                    anyhow!("Recursive YAML aliases are not supported in manifests")
                })?;
                self.alias_bytes += serde_json::to_vec(value)?.len();
                if self.alias_bytes > 64 * 1024 * 1024 {
                    bail!("YAML alias expansion exceeds 64 MiB");
                }
                (value.clone(), 0)
            }
            Event::SequenceStart(anchor, tag) => {
                check_tag(tag.as_ref(), "seq")?;
                let mut values = Vec::new();
                while !matches!(self.events.get(self.index), Some((Event::SequenceEnd, _))) {
                    values.push(self.value(depth + 1)?);
                }
                self.index += 1;
                (Value::Array(values), anchor)
            }
            Event::MappingStart(anchor, tag) => {
                check_tag(tag.as_ref(), "map")?;
                let mut merged = Map::new();
                let mut explicit = Vec::new();
                while !matches!(self.events.get(self.index), Some((Event::MappingEnd, _))) {
                    let merge = matches!(self.events.get(self.index), Some((Event::Scalar(text, TScalarStyle::Plain, _, None), _)) if text == "<<")
                        || matches!(self.events.get(self.index), Some((Event::Scalar(_, _, _, Some(tag)), _)) if tag.handle == "tag:yaml.org,2002:" && tag.suffix == "merge");
                    if merge {
                        self.index += 1;
                        merge_mapping(&mut merged, self.value(depth + 1)?)?;
                    } else {
                        // PyYAML treats the YAML 1.1 value key '=' as a string.
                        let key = if matches!(self.events.get(self.index), Some((Event::Scalar(text, TScalarStyle::Plain, _, None), _)) if text == "=")
                        {
                            self.index += 1;
                            Value::String("=".into())
                        } else {
                            self.value(depth + 1)?
                        };
                        let key = key
                            .as_str()
                            .ok_or_else(|| {
                                anyhow!(
                                    "Manifest mapping keys must be strings at line {}",
                                    marker.line()
                                )
                            })?
                            .to_owned();
                        explicit.push((key, self.value(depth + 1)?));
                    }
                }
                self.index += 1;
                // Explicit values override merged defaults, independent of key order.
                // Duplicate explicit keys retain PyYAML's last-value behavior.
                merged.extend(explicit);
                (Value::Object(merged), anchor)
            }
            _ => bail!("Expected a YAML value at line {}", marker.line()),
        };
        if anchor != 0 {
            self.anchors.insert(anchor, value.clone());
        }
        Ok(value)
    }
}

fn merge_mapping(target: &mut Map<String, Value>, value: Value) -> Result<()> {
    match value {
        Value::Object(map) => target.extend(map),
        Value::Array(values) => {
            for value in values.into_iter().rev() {
                let Value::Object(map) = value else {
                    bail!("YAML merge entries must be mappings");
                };
                target.extend(map);
            }
        }
        _ => bail!("YAML merge value must be a mapping or a sequence of mappings"),
    }
    Ok(())
}

fn check_tag(tag: Option<&Tag>, expected: &str) -> Result<()> {
    if let Some(tag) = tag
        && !(tag.handle.is_empty() && tag.suffix == "!")
        && (tag.handle != "tag:yaml.org,2002:" || tag.suffix != expected)
    {
        bail!("Unsupported YAML tag {}{}", tag.handle, tag.suffix);
    }
    Ok(())
}

fn patterns() -> &'static [Regex; 3] {
    static PATTERNS: OnceLock<[Regex; 3]> = OnceLock::new();
    PATTERNS.get_or_init(|| [
        Regex::new(r"^(?:[-+]?0b[0-1_]+|[-+]?0[0-7_]+|[-+]?(?:0|[1-9][0-9_]*)|[-+]?0x[0-9a-fA-F_]+|[-+]?[1-9][0-9_]*(?::[0-5]?[0-9])+)$").unwrap(),
        Regex::new(r"^(?:[-+]?[0-9][0-9_]*\.[0-9_]*(?:[eE][-+][0-9]+)?|\.[0-9][0-9_]*(?:[eE][-+][0-9]+)?|[-+]?[0-9][0-9_]*(?::[0-5]?[0-9])+\.[0-9_]*|[-+]?\.(?:inf|Inf|INF)|\.(?:nan|NaN|NAN))$").unwrap(),
        Regex::new(r"^(?:[0-9]{4}-[0-9]{2}-[0-9]{2}|[0-9]{4}-[0-9]{1,2}-[0-9]{1,2}(?:[Tt]|[ \t]+)[0-9]{1,2}:[0-9]{2}:[0-9]{2}(?:\.[0-9]*)?(?:[ \t]*(?:Z|[-+][0-9]{1,2}(?::[0-9]{2})?))?)$").unwrap(),
    ])
}

fn scalar(text: &str, style: TScalarStyle, tag: Option<&Tag>) -> Result<Value> {
    let explicit = if let Some(tag) = tag {
        if tag.handle.is_empty() && tag.suffix == "!" {
            None
        } else if tag.handle != "tag:yaml.org,2002:" {
            bail!("Unsupported YAML tag {}{}", tag.handle, tag.suffix);
        } else {
            Some(tag.suffix.as_str())
        }
    } else {
        None
    };
    if explicit == Some("str") || (explicit.is_none() && style != TScalarStyle::Plain) {
        return Ok(Value::String(text.into()));
    }
    if explicit == Some("null")
        || (explicit.is_none() && matches!(text, "" | "~" | "null" | "Null" | "NULL"))
    {
        return Ok(Value::Null);
    }
    let lower;
    let boolean_text = if explicit == Some("bool") {
        lower = text.to_ascii_lowercase();
        &lower
    } else {
        text
    };
    let boolean = match boolean_text {
        "yes" | "Yes" | "YES" | "true" | "True" | "TRUE" | "on" | "On" | "ON" => Some(true),
        "no" | "No" | "NO" | "false" | "False" | "FALSE" | "off" | "Off" | "OFF" => Some(false),
        _ => None,
    };
    if explicit == Some("bool") || (explicit.is_none() && boolean.is_some()) {
        return boolean
            .map(Value::Bool)
            .ok_or_else(|| anyhow!("Invalid YAML boolean: {text}"));
    }
    if explicit == Some("int") || (explicit.is_none() && patterns()[0].is_match(text)) {
        let text = text.replace('_', "");
        let negative = text.starts_with('-');
        let digits = text.strip_prefix(['+', '-']).unwrap_or(&text);
        let integer = if digits.contains(':') {
            digits
                .split(':')
                .try_fold(0i128, |n, part| {
                    n.checked_mul(60)?.checked_add(part.parse::<i128>().ok()?)
                })
                .ok_or_else(|| anyhow!("YAML integer is out of range"))?
        } else {
            let (radix, digits) = if let Some(digits) = digits.strip_prefix("0b") {
                (2, digits)
            } else if let Some(digits) = digits.strip_prefix("0x") {
                (16, digits)
            } else if digits.len() > 1 && digits.starts_with('0') {
                (8, digits.strip_prefix("0o").unwrap_or(&digits[1..]))
            } else {
                (10, digits)
            };
            i128::from_str_radix(digits, radix).context("Invalid YAML integer")?
        };
        let integer = if negative {
            integer
                .checked_neg()
                .context("YAML integer is out of range")?
        } else {
            integer
        };
        return Ok(Value::Number(if integer < 0 {
            Number::from(i64::try_from(integer).context("YAML integer is out of range")?)
        } else {
            Number::from(u64::try_from(integer).context("YAML integer is out of range")?)
        }));
    }
    if explicit == Some("float") || (explicit.is_none() && patterns()[1].is_match(text)) {
        let text = text.replace('_', "");
        let number = if text.contains(':') {
            let sign = if text.starts_with('-') { -1.0 } else { 1.0 };
            text.trim_start_matches(['+', '-'])
                .split(':')
                .try_fold(0.0, |n, part| part.parse::<f64>().map(|v| n * 60.0 + v))?
                * sign
        } else {
            text.parse::<f64>()
                .context("Manifest numbers must be finite")?
        };
        return Number::from_f64(number)
            .map(Value::Number)
            .ok_or_else(|| anyhow!("Manifest numbers must be finite"));
    }
    if explicit == Some("timestamp") || (explicit.is_none() && patterns()[2].is_match(text)) {
        bail!("YAML timestamps are not manifest values; quote '{text}' to use it as a string");
    }
    if explicit.is_none() && matches!(text, "<<" | "=") {
        bail!("YAML value '{text}' must be quoted to use it as a string");
    }
    if let Some(tag) = explicit {
        bail!("Unsupported YAML scalar tag: {tag}");
    }
    Ok(Value::String(text.into()))
}

pub fn stringify(value: &Value) -> Result<String> {
    let mut output = serde_yaml_ng::to_string(value)?;
    fn leaves<'a>(value: &'a Value, values: &mut Vec<Option<&'a str>>) {
        match value {
            Value::Object(map) => {
                for (key, value) in map {
                    values.push(Some(key));
                    leaves(value, values);
                }
            }
            Value::Array(array) => {
                for value in array {
                    leaves(value, values);
                }
            }
            Value::String(text) => values.push(Some(text)),
            _ => values.push(None),
        }
    }
    let mut values = Vec::new();
    leaves(value, &mut values);
    let mut values = values.into_iter();
    let mut replacements = Vec::new();
    // yaml-rust2 advances its marker once per character, including Unicode.
    // String slicing needs byte offsets instead.
    let offsets: Vec<_> = output.char_indices().map(|(index, _)| index).collect();
    for (event, marker) in events(&output)? {
        if let Event::Scalar(text, style, _, _) = event {
            let original = values
                .next()
                .ok_or_else(|| anyhow!("YAML emitter scalar count mismatch"))?;
            if let Some(original) = original
                && style == TScalarStyle::Plain
                && !matches!(scalar(original, style, None), Ok(Value::String(_)))
            {
                let start = *offsets
                    .get(marker.index())
                    .ok_or_else(|| anyhow!("YAML emitter scalar offset mismatch"))?;
                if output.get(start..start + text.len()) != Some(&text) {
                    bail!("YAML emitter scalar offset mismatch");
                }
                replacements.push((
                    start..start + text.len(),
                    format!("'{}'", original.replace('\'', "''")),
                ));
            }
        }
    }
    if values.next().is_some() {
        bail!("YAML emitter scalar count mismatch");
    }
    for (range, text) in replacements.into_iter().rev() {
        output.replace_range(range, &text);
    }
    Ok(output)
}

#[cfg(test)]
#[path = "../tests/rust_unit/yaml.rs"]
mod tests;

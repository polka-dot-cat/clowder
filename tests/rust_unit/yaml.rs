
use super::*;
use serde_json::json;

#[test]
fn preserves_yaml11_scalars_and_quoted_strings() {
    assert_eq!(
            parse(
                "[yes, NO, On, off, y, n, 012, 0b10, 0x10, 1:20, 1.2e+3, 1e3, 'yes', !!str false, !!bool YeS, 2026-1-2]"
            )
            .unwrap(),
            json!([
                true, false, true, false, "y", "n", 10, 2, 16, 80, 1200.0, "1e3", "yes", "false", true, "2026-1-2"
            ])
        );
}

#[test]
fn expands_anchors_and_merges_with_explicit_values_winning() {
    let value = parse("base: &base {branch: main, remote: origin}\nother: &other {branch: other}\nproject: {branch: feature, <<: [*base, *other]}\n").unwrap();
    assert_eq!(
        value["project"],
        json!({"branch":"feature","remote":"origin"})
    );
    assert_eq!(
        parse("name: old\nname: new\n").unwrap(),
        json!({"name":"new"})
    );
    assert!(parse("a: &a [*a]").is_err());
    assert!(parse("a: {<<: 42}").is_err());
    assert!(parse("---\na: 1\n---\nb: 2").is_err());
}

#[test]
fn saved_strings_keep_their_type_with_python_scalar_rules() {
    let value = json!({"name":"猫", "clowder": {"on": ["yes", "012", "2026-10-24", "1:20", "1e3", "normal", "<<", "="]}});
    let text = stringify(&value).unwrap();
    assert!(text.contains("'on':"));
    assert!(text.contains("- 'yes'"));
    assert_eq!(parse(&text).unwrap(), value);
}

#[test]
fn matches_captured_python_yaml_values() {
    let fixture: Value =
        serde_json::from_str(include_str!("../data/rust/yaml-compatibility.json")).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let text = case["yaml"].as_str().unwrap();
        let result = parse(text);
        if case["error"] == true {
            assert!(result.is_err(), "Accepted invalid YAML: {text}");
        } else {
            let value = result.unwrap_or_else(|error| panic!("{text:?}: {error}"));
            assert_eq!(value, case["value"], "YAML: {text}");
            assert_eq!(parse(&stringify(&value).unwrap()).unwrap(), value);
        }
    }
}

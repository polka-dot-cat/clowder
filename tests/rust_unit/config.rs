
use super::*;

#[test]
fn matches_python_config_parser_values_errors_and_serialization() {
    let fixture: Value =
        serde_json::from_str(include_str!("../data/rust/config-compatibility.json")).unwrap();
    let directory = tempfile::TempDir::new().unwrap();
    let workspace = Workspace {
        root: directory.path().into(),
        found: true,
    };
    for case in fixture["cases"].as_array().unwrap() {
        let text = case["ini"].as_str().unwrap();
        let result = Config::parse(text);
        if case["parse_error"] == true {
            assert!(result.is_err(), "Accepted invalid INI: {text}");
            continue;
        }
        let config = result.unwrap_or_else(|e| panic!("{text:?}: {e}"));
        for query in case["queries"].as_array().unwrap() {
            let actual = config.get(
                query["section"].as_str().unwrap(),
                query["key"].as_str().unwrap(),
            );
            if query["error"] == true {
                assert!(actual.is_err(), "{text:?}, query: {query}");
            } else {
                assert_eq!(
                    actual.unwrap().as_deref(),
                    query["value"].as_str(),
                    "{text:?}, query: {query}"
                );
            }
        }
        config.save(&workspace).unwrap();
        assert_eq!(
            fs::read_to_string(workspace.root.join("clowder.config")).unwrap(),
            case["saved"].as_str().unwrap(),
            "{text:?}"
        );
    }
}

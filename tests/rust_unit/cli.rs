use super::*;

fn args(tokens: &[&str]) -> Result<Args> {
    parse(tokens.iter().map(|s| s.to_string()).collect()).map(Option::unwrap)
}

#[test]
fn preserves_quoted_shell_commands_and_combined_flags() {
    let result = args(&["forall", "-ij4", "printf '%s' \"$PROJECT_NAME\"", "cats"]).unwrap();
    assert!(result.flag("ignore_errors"));
    assert_eq!(result.number("jobs"), Some(4));
    assert_eq!(result.one("command"), Some("printf '%s' \"$PROJECT_NAME\""));
    assert_eq!(result.many("projects"), vec!["cats"]);
}

#[test]
fn validates_values_conflicts_and_missing_arguments() {
    assert!(args(&["herd", "-b", "main", "-t", "v1"]).is_err());
    assert!(args(&["herd", "--jobs", "many"]).is_err());
    assert!(args(&["config", "set", "protocol", "ftp"]).is_err());
    assert!(args(&["checkout"]).is_err());
    assert_eq!(
        args(&["config", "set", "jobs", "-1"])
            .unwrap()
            .number("jobs"),
        Some(-1)
    );
}

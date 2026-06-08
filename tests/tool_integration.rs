use tempfile::TempDir;

#[test]
fn test_bash_echo() {
    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg("echo hello")
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&output.stdout).contains("hello"));
}

#[test]
fn test_file_read_write_roundtrip() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.txt");
    std::fs::write(&path, "line1\nline2\nline3\n").unwrap();
    let content = std::fs::read_to_string(&path).unwrap();
    assert_eq!(content.lines().count(), 3);
}

#[test]
fn test_regex_and_glob() {
    let re = regex::Regex::new(r"fn\s+\w+").unwrap();
    assert!(re.is_match("fn hello() {}"));
    let glob = globset::Glob::new("*.rs").unwrap();
    assert!(glob.compile_matcher().is_match("main.rs"));
}

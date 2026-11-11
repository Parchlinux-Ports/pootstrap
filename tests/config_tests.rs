use std::fs;
use tempfile::tempdir;

#[test]
fn test_config_generation_and_saving() {
    let dir = tempdir().expect("Failed to create tempdir");
    let config_path = dir.path().join("config.toml");

    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "--config",
            config_path.to_str().unwrap(),
            "init",
            "--device",
            "purism-librem5",
            "--flavour",
            "plasma-mobile",
        ])
        .output()
        .expect("Failed to run init");

    assert!(output.status.success());
    assert!(config_path.exists());

    let content = fs::read_to_string(&config_path).expect("Failed to read saved config");
    assert!(content.contains("purism-librem5"));
    assert!(content.contains("plasma-mobile"));
    assert!(content.contains("parch-mobile"));
}

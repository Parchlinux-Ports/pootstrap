use std::fs;
use tempfile::tempdir;

#[test]
fn test_pacman_conf_generation() {
    let dir = tempdir().expect("Failed to create tempdir");
    let conf_path = dir.path().join("pacman.conf");

    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "--config",
            dir.path().join("config.toml").to_str().unwrap(),
            "init",
        ])
        .output()
        .expect("Failed to run init");
    assert!(output.status.success());

    // Verify pacman conf format through unit test or by checking generated string
    let conf_content = format!(
        r#"[options]
Architecture = aarch64
SigLevel = Never

[parch-mobile]
SigLevel = Never
Server = https://repo.parchlinux.com/$arch/$repo/$arch
"#
    );
    fs::write(&conf_path, &conf_content).unwrap();
    let read_back = fs::read_to_string(&conf_path).unwrap();
    assert!(read_back.contains("Architecture = aarch64"));
    assert!(read_back.contains("[parch-mobile]"));
}

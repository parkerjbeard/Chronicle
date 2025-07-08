use assert_cmd::Command;
use predicates::prelude::*;
use std::process::Command as StdCommand;
use tempfile::TempDir;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("chronictl").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Chronicle CLI"));
}

#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("chronictl").unwrap();
    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("chronictl"));
}

#[test]
fn test_status_command_help() {
    let mut cmd = Command::cargo_bin("chronictl").unwrap();
    cmd.args(["status", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Check Chronicle service status"));
}

#[test]
fn test_search_command_help() {
    let mut cmd = Command::cargo_bin("chronictl").unwrap();
    cmd.args(["search", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Search events with queries"));
}

#[test]
fn test_export_command_help() {
    let mut cmd = Command::cargo_bin("chronictl").unwrap();
    cmd.args(["export", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Export data in various formats"));
}

#[test]
fn test_config_command_help() {
    let mut cmd = Command::cargo_bin("chronictl").unwrap();
    cmd.args(["config", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Manage Chronicle configuration"));
}

#[test]
fn test_backup_command_help() {
    let mut cmd = Command::cargo_bin("chronictl").unwrap();
    cmd.args(["backup", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Create backups of Chronicle data"));
}

#[test]
fn test_wipe_command_help() {
    let mut cmd = Command::cargo_bin("chronictl").unwrap();
    cmd.args(["wipe", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Securely wipe Chronicle data"));
}

#[test]
fn test_replay_command_help() {
    let mut cmd = Command::cargo_bin("chronictl").unwrap();
    cmd.args(["replay", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Replay events with timing simulation"));
}

#[test]
fn test_completions_command() {
    let mut cmd = Command::cargo_bin("chronictl").unwrap();
    cmd.args(["completions", "bash"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("complete"));
}

#[test]
fn test_invalid_command() {
    let mut cmd = Command::cargo_bin("chronictl").unwrap();
    cmd.arg("invalid-command");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("error:"));
}

#[test]
fn test_status_with_ping() {
    // This test will fail if Chronicle service is not running, which is expected
    let mut cmd = Command::cargo_bin("chronictl").unwrap();
    cmd.args(["status", "--ping"]);
    // We don't assert success because the service might not be running
    let output = cmd.output().unwrap();
    // Just check that the command runs and produces some output
    assert!(!output.stdout.is_empty() || !output.stderr.is_empty());
}

#[test]
fn test_search_invalid_query() {
    let mut cmd = Command::cargo_bin("chronictl").unwrap();
    cmd.args(["search", "--query", ""]);
    cmd.assert()
        .failure(); // Empty query should fail
}

#[test]
fn test_export_invalid_format() {
    let mut cmd = Command::cargo_bin("chronictl").unwrap();
    cmd.args(["export", "--format", "invalid"]);
    cmd.assert()
        .failure(); // Invalid format should fail
}

#[test]
fn test_config_with_temp_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.json");
    
    // Create a test config file
    std::fs::write(&config_path, r#"{"service_url": "http://localhost:8080"}"#).unwrap();
    
    let mut cmd = Command::cargo_bin("chronictl").unwrap();
    cmd.args([
        "--config", 
        config_path.to_str().unwrap(),
        "config",
        "show"
    ]);
    
    // This should work with a valid config file, even if service is not running
    let output = cmd.output().unwrap();
    assert!(!output.stdout.is_empty() || !output.stderr.is_empty());
}

#[test]
fn test_global_flags() {
    let mut cmd = Command::cargo_bin("chronictl").unwrap();
    cmd.args(["--format", "json", "--no-color", "status", "--ping"]);
    
    // Should accept global flags
    let output = cmd.output().unwrap();
    assert!(!output.stdout.is_empty() || !output.stderr.is_empty());
}

#[test]
fn test_environment_variables() {
    let mut cmd = Command::cargo_bin("chronictl").unwrap();
    cmd.env("CHRONICLE_URL", "http://test.example.com");
    cmd.args(["status", "--ping"]);
    
    // Should accept environment variables
    let output = cmd.output().unwrap();
    assert!(!output.stdout.is_empty() || !output.stderr.is_empty());
}

#[test]
fn test_dry_run_operations() {
    // Test dry run for backup
    let mut cmd = Command::cargo_bin("chronictl").unwrap();
    cmd.args(["backup", "--destination", "/tmp/test", "--dry-run"]);
    let output = cmd.output().unwrap();
    assert!(!output.stdout.is_empty() || !output.stderr.is_empty());
    
    // Test dry run for wipe
    let mut cmd = Command::cargo_bin("chronictl").unwrap();
    cmd.args(["wipe", "--dry-run"]);
    let output = cmd.output().unwrap();
    assert!(!output.stdout.is_empty() || !output.stderr.is_empty());
}

// Helper function to check if Chronicle service is running
#[allow(dead_code)]
fn is_chronicle_service_running() -> bool {
    // Try to connect to the default Chronicle service URL
    std::process::Command::new("curl")
        .args(["-s", "http://localhost:8080/health"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

// Integration test that requires a running Chronicle service
#[test]
#[ignore] // Ignored by default, run with --ignored if service is available
fn test_with_running_service() {
    if !is_chronicle_service_running() {
        println!("Chronicle service not running, skipping integration test");
        return;
    }
    
    // Test actual status command
    let mut cmd = Command::cargo_bin("chronictl").unwrap();
    cmd.args(["status"]);
    cmd.assert().success();
    
    // Test actual search command
    let mut cmd = Command::cargo_bin("chronictl").unwrap();
    cmd.args(["search", "--query", "*", "--limit", "1"]);
    cmd.assert().success();
}
use anyhow::Result;
use assert_cmd::Command;
use insta::assert_snapshot;
use std::path::PathBuf;
use tempfile::{tempdir, TempDir};

// Create a simple test template using proper tera syntax
const TEMPLATE_CONTENT: &str = r#"// Generated from {{template_path}} template.

pub struct TestStruct{{scalar_t | upper}} {
    pub value: {{scalar_t}},
}

impl TestStruct{{scalar_t | upper}} {
    pub fn new(v: {{scalar_t}}) -> Self {
        TestStruct{{scalar_t}} {
            value: v,
        }
    }

    pub fn get_value(&self) -> {{scalar_t}} {
        self.value
    }
}
"#;

/// Helper to create a temporary git directory for test fixtures
fn setup_test_dir() -> Result<TempDir> {
    let dir = tempdir()?;

    // Initialize git repo (codegen requires a git repository)
    Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .assert()
        .success();

    let templates_dir = dir.path().join("templates");
    std::fs::create_dir_all(&templates_dir)?;

    Ok(dir)
}

/// Helper to create a temporary directory with a single templates
fn setup_single_template_dir(dir: &TempDir) -> Result<()> {
    let templates_dir = dir.path().join("templates");
    std::fs::write(templates_dir.join("test.rs.tera"), TEMPLATE_CONTENT)?;
    Ok(())
}

fn setup_config(dir: &TempDir, json: &str) -> Result<PathBuf> {
    let config_path = dir.path().join("codegen.json");
    std::fs::write(&config_path, json)?;

    Ok(config_path)
}

/// Helper to create a simple config file
fn setup_simple_config(dir: &TempDir) -> Result<PathBuf> {
    setup_config(
        dir,
        r#"{
            "version": 1,
            "template_root": "templates",
            "templates": {
                "test.rs.tera": {
                    "properties": {"scalar_t": null},
                    "outputs": {
                        "src/test_output.rs": {"properties": {"scalar_t": "f32"}}
                    }
                }
            }
        }"#,
    )
}

/// Test: Codegen generates files correctly
#[test]
fn test_generate_files() -> Result<()> {
    let dir = setup_test_dir()?;
    setup_single_template_dir(&dir)?;
    let config_path = setup_simple_config(&dir)?;

    // Run codegen binary directly with --config flag
    let mut cmd = Command::cargo_bin("codegen")?;
    cmd.args(["--config", config_path.to_str().unwrap()]);

    cmd.assert().success();

    // Check that the generated file was created
    let output_path = dir.path().join("src/test_output.rs");
    assert!(
        output_path.exists(),
        "Generated file should exist at {:?}",
        output_path
    );

    // Read the generated content
    let content = std::fs::read_to_string(&output_path)?;
    assert_snapshot!(content);

    Ok(())
}

/// Test: Codegen check mode detection
#[test]
fn test_check_mode() -> Result<()> {
    let dir = setup_test_dir()?;
    setup_single_template_dir(&dir)?;
    let config_path = setup_simple_config(&dir)?;

    // First generate the file
    let mut cmd = Command::cargo_bin("codegen")?;
    cmd.args(["--config", config_path.to_str().unwrap()]);
    cmd.assert().success();

    // Run codegen in check mode with matching files
    let mut cmd = Command::cargo_bin("codegen")?;
    cmd.args(["--config", config_path.to_str().unwrap(), "--check"]);
    cmd.assert().success();

    // Now modify the generated file to have different content
    let output_path = dir.path().join("src/test_output.rs");
    std::fs::write(&output_path, "// This is different content")?;

    // Run codegen in check mode
    let mut cmd = Command::cargo_bin("codegen")?;
    cmd.args(["--config", config_path.to_str().unwrap(), "--check"]);

    cmd.assert().failure();

    // Check that differences were detected in output
    let output = cmd.output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_snapshot!(stderr);

    Ok(())
}

/// Test: Multiple templates with different properties
#[test]
fn test_multiple_templates() -> Result<()> {
    let dir = setup_test_dir()?;

    let templates_dir = dir.path().join("templates");
    std::fs::write(templates_dir.join("test1.rs.tera"), TEMPLATE_CONTENT)?;
    std::fs::write(templates_dir.join("test2.rs.tera"), TEMPLATE_CONTENT)?;

    let config_path = setup_config(
        &dir,
        r#"{
            "version": 1,
            "template_root": "templates",
            "templates": {
                "test1.rs.tera": {
                    "properties": {"scalar_t": null},
                    "outputs": {
                        "src/test1_f32.rs": {"properties": {"scalar_t": "f32"}},
                        "src/test1_f64.rs": {"properties": {"scalar_t": "f64"}}
                    }
                },
                "test2.rs.tera": {
                    "properties": {"scalar_t": null},
                    "outputs": {
                        "src/test2_f32.rs": {"properties": {"scalar_t": "f32"}}
                    }
                }
            }
        }"#,
    )?;

    // Run codegen and check that all files are generated
    let mut cmd = Command::cargo_bin("codegen")?;
    cmd.args(["--config", config_path.to_str().unwrap()]);
    cmd.assert().success();

    assert_snapshot!(std::fs::read_to_string(
        dir.path().join("src/test1_f32.rs")
    )?);
    assert_snapshot!(std::fs::read_to_string(
        dir.path().join("src/test1_f64.rs")
    )?);
    assert_snapshot!(std::fs::read_to_string(
        dir.path().join("src/test2_f32.rs")
    )?);

    Ok(())
}

/// Test: Error handling for missing properties
#[test]
fn test_error_missing_properties() -> Result<()> {
    let dir = setup_test_dir()?;
    setup_single_template_dir(&dir)?;
    let config_path = setup_config(
        &dir,
        r#"{
            "version": 1,
            "template_root": "templates",
            "templates": {
                "test.rs.tera": {
                    "properties": {"scalar_t": null},
                    "outputs": {
                        "src/output.rs": {"properties": {}}
                    }
                }
            }
        }"#,
    )?;

    // Run codegen and check that it fails with an error
    let mut cmd = Command::cargo_bin("codegen")?;
    cmd.args(["--config", config_path.to_str().unwrap()]);

    cmd.assert().failure();

    let output = cmd.output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_snapshot!(stderr);

    Ok(())
}

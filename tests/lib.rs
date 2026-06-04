use anyhow::Result;
use assert_cmd::Command;
use tempfile::{tempdir, TempDir};

/// Helper to create a temporary directory with test fixtures
#[cfg(test)]
fn setup_test_dir() -> Result<TempDir> {
    let dir = tempdir()?;

    // Initialize git repo (codegen requires a git repository)
    Command::new("git")
        .args(&["init"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Create templates directory with sample template
    let templates_dir = dir.path().join("templates");
    std::fs::create_dir_all(&templates_dir)?;

    // Create a simple test template using proper tera syntax
    let template_content = r#"// Generated from {{template_path}} template.

pub struct TestStruct{{scalar_t}} {
    pub value: {{scalar_t}},
}

impl TestStruct{{scalar_t}} {
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

    std::fs::write(templates_dir.join("test.rs.tera"), template_content)?;

    Ok(dir)
}

/// Helper to create a temporary directory with multiple templates
#[cfg(test)]
fn setup_multi_template_dir() -> Result<TempDir> {
    let dir = tempdir()?;

    // Initialize git repo (codegen requires a git repository)
    Command::new("git")
        .args(&["init"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Create templates directory with sample templates
    let templates_dir = dir.path().join("templates");
    std::fs::create_dir_all(&templates_dir)?;

    // Create a simple test template using proper tera syntax
    let template_content = r#"// Generated from {{template_path}} template.

pub struct TestStruct{{scalar_t}} {
    pub value: {{scalar_t}},
}

impl TestStruct{{scalar_t}} {
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

    std::fs::write(templates_dir.join("test1.rs.tera"), template_content)?;
    std::fs::write(templates_dir.join("test2.rs.tera"), template_content)?;

    Ok(dir)
}

/// Test: Codegen generates files correctly
#[test]
fn test_generate_files() -> Result<()> {
    let dir = setup_test_dir()?;
    let config_path = dir.path().join("codegen.json");

    let json = r#"{
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
        }"#;
    std::fs::write(&config_path, json)?;

    // Run codegen binary directly with --config flag
    let mut cmd = Command::cargo_bin("codegen")?;
    cmd.args(&["-c", config_path.to_str().unwrap()]);

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

    // Verify it contains expected content
    assert!(
        content.contains("pub struct TestStructf32"),
        "Content should contain 'pub struct TestStructf32', got: {}",
        content
    );
    assert!(content.contains("// Generated from test.rs.tera template"));

    Ok(())
}

/// Test: Codegen check mode detects differences
#[test]
fn test_check_mode_detects_differences() -> Result<()> {
    let dir = setup_test_dir()?;
    let config_path = dir.path().join("codegen.json");

    let json = r#"{
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
        }"#;
    std::fs::write(&config_path, json)?;

    // First generate the file
    let mut cmd = Command::cargo_bin("codegen")?;
    cmd.args(&["-c", config_path.to_str().unwrap()]);
    cmd.assert().success();

    // Now modify the generated file to have different content
    let output_path = dir.path().join("src/test_output.rs");
    std::fs::write(&output_path, "// This is different content")?;

    // Run codegen in check mode
    let mut cmd = Command::cargo_bin("codegen")?;
    cmd.args(&["-c", config_path.to_str().unwrap(), "--check"]);

    cmd.assert().failure();

    // Check that differences were detected in output
    let output = cmd.output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("files were different"),
        "Expected 'files were different' in stderr, got: {}",
        stderr
    );

    Ok(())
}

/// Test: Codegen check mode with matching files passes
#[test]
fn test_check_mode_matching_files() -> Result<()> {
    let dir = setup_test_dir()?;
    let config_path = dir.path().join("codegen.json");

    let json = r#"{
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
        }"#;
    std::fs::write(&config_path, json)?;

    // Generate the file first
    let mut cmd = Command::cargo_bin("codegen")?;
    cmd.args(&["-c", config_path.to_str().unwrap()]);
    cmd.assert().success();

    // Run codegen in check mode with matching files
    let mut cmd = Command::cargo_bin("codegen")?;
    cmd.args(&["-c", config_path.to_str().unwrap(), "--check"]);

    cmd.assert().success();

    // Check that no differences were found
    let output = cmd.output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("files were different"),
        "Expected no 'files were different' in stderr, got: {}",
        stderr
    );

    Ok(())
}

/// Test: Multiple templates with different properties
#[test]
fn test_multiple_templates() -> Result<()> {
    let dir = setup_multi_template_dir()?;
    let config_path = dir.path().join("codegen.json");

    let json = r#"{
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
        }"#;
    std::fs::write(&config_path, json)?;

    // Run codegen and check that all files are generated
    let mut cmd = Command::cargo_bin("codegen")?;
    cmd.args(&["-c", config_path.to_str().unwrap()]);
    cmd.assert().success();

    assert!(dir.path().join("src/test1_f32.rs").exists());
    assert!(dir.path().join("src/test1_f64.rs").exists());
    assert!(dir.path().join("src/test2_f32.rs").exists());

    Ok(())
}

/// Test: Error handling for missing properties
#[test]
fn test_error_missing_properties() -> Result<()> {
    let dir = setup_test_dir()?;
    let config_path = dir.path().join("codegen.json");

    // Missing scalar_t in output properties but present in template
    let json = r#"{
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
        }"#;
    std::fs::write(&config_path, json)?;

    // Run codegen and check that it fails with an error
    let mut cmd = Command::cargo_bin("codegen")?;
    cmd.args(&["-c", config_path.to_str().unwrap()]);

    cmd.assert().failure();

    let output = cmd.output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Missing property override"));

    Ok(())
}

/// Test: Template path is correctly set in generated files
#[test]
fn test_template_path_in_generated_files() -> Result<()> {
    let dir = setup_test_dir()?;
    let config_path = dir.path().join("codegen.json");

    let json = r#"{
            "version": 1,
            "template_root": "templates",
            "templates": {
                "test.rs.tera": {
                    "properties": {"scalar_t": null},
                    "outputs": {
                        "src/output.rs": {"properties": {"scalar_t": "f32"}}
                    }
                }
            }
        }"#;
    std::fs::write(&config_path, json)?;

    // Run codegen
    let mut cmd = Command::cargo_bin("codegen")?;
    cmd.args(&["-c", config_path.to_str().unwrap()]);
    cmd.assert().success();

    // Read the generated file
    let output_path = dir.path().join("src/output.rs");
    let content = std::fs::read_to_string(&output_path)?;

    // Verify it contains the template path comment
    assert!(content.contains("// Generated from test.rs.tera template"));

    Ok(())
}


use std::fs;
use tempfile::TempDir;
use uv_migrator::utils::pyproject::append_tool_sections;

/// Helper function to create a temporary test directory with pyproject files.
///
/// # Arguments
///
/// * `old_content` - Content for old.pyproject.toml
/// * `new_content` - Content for pyproject.toml
///
/// # Returns
///
/// A tuple containing the temporary directory and its path
fn setup_test_files(old_content: &str, new_content: &str) -> (TempDir, std::path::PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().to_path_buf();

    fs::write(project_dir.join("old.pyproject.toml"), old_content).unwrap();
    fs::write(project_dir.join("pyproject.toml"), new_content).unwrap();

    (temp_dir, project_dir)
}

#[test]
fn test_append_tool_sections() {
    let old_content = r#"
[tool.poetry]
name = "test"
version = "0.1.0"

[tool.black]
line-length = 100
target-version = ["py37"]

[tool.isort]
profile = "black"
"#;

    let new_content = r#"
[project]
name = "test"
version = "0.1.0"
description = "A test project"
"#;

    let (_temp_dir, project_dir) = setup_test_files(old_content, new_content);
    append_tool_sections(&project_dir).unwrap();

    let result = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();

    // Verify tool sections were copied correctly
    assert!(
        result.contains("[tool.black]"),
        "black section should be present"
    );
    assert!(
        result.contains("[tool.isort]"),
        "isort section should be present"
    );
    assert!(
        result.contains("line-length = 100"),
        "black settings should be preserved"
    );
    assert!(
        result.contains("profile = \"black\""),
        "isort settings should be preserved"
    );

    // Verify poetry section was not copied
    assert!(
        !result.contains("[tool.poetry]"),
        "poetry section should not be present"
    );

    // Verify [tool] section behavior
    let tool_count = result.matches("[tool]").count();
    assert!(tool_count <= 1, "Should not have multiple [tool] sections");
    if tool_count == 1 {
        // If we have a [tool] section, ensure it's not empty by checking for its subsections
        let tool_index = result.find("[tool]").unwrap();
        let next_section = result[tool_index..]
            .find("\n[")
            .unwrap_or(result.len() - tool_index);
        let tool_content = &result[tool_index..tool_index + next_section];
        assert!(
            tool_content.contains("[tool.black]") || tool_content.contains("[tool.isort]"),
            "Empty [tool] section should not be present"
        );
    }
}

#[test]
fn test_append_tool_sections_with_existing() {
    let old_content = r#"
[tool.poetry]
name = "test"
version = "0.1.0"

[tool.black]
line-length = 100

[tool.isort]
profile = "black"
"#;

    let new_content = r#"
[project]
name = "test"
version = "0.1.0"

[tool.black]
line-length = 88
"#;

    let (_temp_dir, project_dir) = setup_test_files(old_content, new_content);
    append_tool_sections(&project_dir).unwrap();

    let result = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();

    // Verify existing tool.black was preserved
    let black_section = result.find("[tool.black]").unwrap();
    let next_section = result[black_section..]
        .find("\n[")
        .unwrap_or(result.len() - black_section);
    let black_content = &result[black_section..black_section + next_section];
    assert!(
        black_content.contains("line-length = 88"),
        "Existing black configuration should be preserved"
    );

    // Verify isort was copied
    assert!(
        result.contains("[tool.isort]"),
        "isort section should be present"
    );
    assert!(
        result.contains("profile = \"black\""),
        "isort settings should be preserved"
    );

    // Verify no empty sections
    assert!(
        !result.matches("[tool]").any(|_| true),
        "Should not have empty [tool] section"
    );
}

#[test]
fn test_preserve_formatting() {
    let old_content = r#"
[tool.black]
line-length = 100  # Custom line length
target-version = [
    "py37",
    "py38",
]  # Supported versions

[tool.isort]
profile = "black"  # Match black
"#;

    let new_content = "[project]\nname = \"test\"\n";

    let (_temp_dir, project_dir) = setup_test_files(old_content, new_content);
    append_tool_sections(&project_dir).unwrap();

    let result = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();

    // Verify comments and formatting were preserved
    assert!(result.contains("line-length = 100  # Custom line length"));
    assert!(result.contains("profile = \"black\"  # Match black"));
    assert!(result.contains(
        r#"target-version = [
    "py37",
    "py38",
]  # Supported versions"#
    ));
}

#[test]
fn test_no_old_pyproject() {
    let new_content = r#"
[project]
name = "test"
version = "0.1.0"
description = "A test project"
"#;

    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().to_path_buf();
    fs::write(project_dir.join("pyproject.toml"), new_content).unwrap();

    let result = append_tool_sections(&project_dir);
    assert!(
        result.is_ok(),
        "Should handle missing old.pyproject.toml gracefully"
    );

    let final_content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();
    assert_eq!(
        final_content, new_content,
        "Content should remain unchanged"
    );
    assert!(
        !final_content.contains("[tool]"),
        "Should not have empty [tool] section"
    );
}

#[test]
fn test_nested_tool_sections() {
    let old_content = r#"
[tool.poetry]
name = "test"
version = "0.1.0"

[tool.black]
line-length = 100

[tool.pytest.ini_options]
minversion = "6.0"
addopts = "-ra -q"
testpaths = ["tests"]
"#;

    let new_content = r#"
[project]
name = "test"
version = "0.1.0"
"#;

    let (_temp_dir, project_dir) = setup_test_files(old_content, new_content);
    append_tool_sections(&project_dir).unwrap();

    let result = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();

    // Verify nested sections were copied correctly
    assert!(
        result.contains("[tool.pytest.ini_options]"),
        "Nested pytest section should be present"
    );
    assert!(
        result.contains("minversion = \"6.0\""),
        "Nested section content should be preserved"
    );
    assert!(
        result.contains("testpaths = [\"tests\"]"),
        "Array values should be preserved"
    );

    // Verify no empty sections
    assert!(
        !result.matches("[tool]").any(|_| true),
        "Should not have empty [tool] section"
    );
    let tool_sections = result.matches("[tool.").count();
    assert!(tool_sections > 0, "Should have non-empty tool sections");
}

#[test]
fn test_empty_nested_sections() {
    let old_content = r#"
[tool.poetry]
name = "test"

[tool.black]
line-length = 100

[tool.pytest]

[tool.pytest.ini_options]
"#;

    let new_content = r#"
[project]
name = "test"
version = "0.1.0"
"#;

    let (_temp_dir, project_dir) = setup_test_files(old_content, new_content);
    append_tool_sections(&project_dir).unwrap();

    let result = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();

    // Verify only non-empty sections were copied
    assert!(
        result.contains("[tool.black]"),
        "Non-empty black section should be present"
    );
    assert!(
        !result.contains("[tool.pytest]"),
        "Empty pytest section should not be present"
    );
    assert!(
        !result.contains("[tool.pytest.ini_options]"),
        "Empty nested section should not be present"
    );

    // Verify no empty tool section
    assert!(
        !result.matches("[tool]").any(|_| true),
        "Should not have empty [tool] section"
    );
}

#[test]
fn test_no_empty_tool_section() {
    let old_content = r#"
[tool.poetry]
name = "test"
version = "0.1.0"
"#;

    let new_content = r#"
[project]
name = "test"
version = "0.1.0"
description = "A test project"
"#;

    let (_temp_dir, project_dir) = setup_test_files(old_content, new_content);
    append_tool_sections(&project_dir).unwrap();

    let result = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();
    assert!(
        !result.contains("[tool]"),
        "Empty [tool] section should not be present"
    );
}

#[test]
fn test_no_empty_tool_section_after_cleanup() {
    let old_content = r#"
[tool.poetry]
name = "test"
version = "0.1.0"

[tool.black]
"#;

    let new_content = r#"
[project]
name = "test"
version = "0.1.0"
"#;

    let (_temp_dir, project_dir) = setup_test_files(old_content, new_content);
    append_tool_sections(&project_dir).unwrap();

    let result = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();
    assert!(
        !result.contains("[tool]"),
        "Empty [tool] section should not be present after cleanup"
    );
    assert!(
        !result.contains("[tool.black]"),
        "Empty black section should be cleaned up"
    );
}

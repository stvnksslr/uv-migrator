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

/// Test basic tool section appending functionality.
///
/// This test verifies that:
/// 1. Tool sections are correctly copied from old to new pyproject.toml
/// 2. Poetry sections are properly excluded
/// 3. Tool section values are preserved accurately
/// 4. No duplicate or empty tool sections are created
///
/// # Test Setup
/// Creates two TOML files:
/// - old.pyproject.toml with poetry, black, and isort sections
/// - pyproject.toml with basic project configuration
///
/// # Verification Steps
/// 1. Verifies black and isort sections are copied
/// 2. Confirms poetry section is not copied
/// 3. Validates section content preservation
/// 4. Checks for proper tool section structure
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

/// Test handling of existing tool sections during append operation.
///
/// This test verifies that:
/// 1. Existing tool sections in the target file are preserved
/// 2. Non-conflicting sections from old file are appended
/// 3. Existing section values are not overwritten
/// 4. Section order is maintained appropriately
///
/// # Test Setup
/// Creates two TOML files:
/// - old.pyproject.toml with poetry, black, and isort sections
/// - pyproject.toml with existing black section
///
/// # Verification Steps
/// 1. Confirms existing black configuration is preserved
/// 2. Verifies isort section is properly copied
/// 3. Checks no empty sections are created
/// 4. Validates overall section structure
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

    let black_section = result.find("[tool.black]").unwrap();
    let next_section = result[black_section..]
        .find("\n[")
        .unwrap_or(result.len() - black_section);
    let black_content = &result[black_section..black_section + next_section];
    assert!(
        black_content.contains("line-length = 88"),
        "Existing black configuration should be preserved"
    );

    assert!(
        result.contains("[tool.isort]"),
        "isort section should be present"
    );
    assert!(
        result.contains("profile = \"black\""),
        "isort settings should be preserved"
    );

    assert!(
        !result.matches("[tool]").any(|_| true),
        "Should not have empty [tool] section"
    );
}

/// Test preservation of TOML formatting and comments.
///
/// This test verifies that:
/// 1. Inline comments are preserved
/// 2. Multi-line formatting is maintained
/// 3. Array formatting and indentation is kept
/// 4. Section-level comments are retained
///
/// # Test Setup
/// Creates TOML files with:
/// - Inline comments
/// - Multi-line arrays
/// - Section comments
/// - Various formatting styles
///
/// # Verification Steps
/// 1. Checks inline comment preservation
/// 2. Verifies multi-line array formatting
/// 3. Validates section comment retention
/// 4. Confirms overall formatting structure
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

    assert!(result.contains("line-length = 100  # Custom line length"));
    assert!(result.contains("profile = \"black\"  # Match black"));
    assert!(result.contains(
        r#"target-version = [
    "py37",
    "py38",
]  # Supported versions"#
    ));
}

/// Test handling of missing old pyproject.toml file.
///
/// This test verifies that:
/// 1. Missing old.pyproject.toml is handled gracefully
/// 2. Existing pyproject.toml remains unchanged
/// 3. No empty sections are created
/// 4. Operation completes successfully
///
/// # Test Setup
/// Creates only a new pyproject.toml file without old.pyproject.toml
///
/// # Verification Steps
/// 1. Confirms operation succeeds without error
/// 2. Verifies content remains unchanged
/// 3. Checks no empty sections are added
/// 4. Validates file integrity
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

/// Test handling of nested tool sections.
///
/// This test verifies that:
/// 1. Nested tool sections are correctly copied
/// 2. Section hierarchy is preserved
/// 3. Nested values are maintained accurately
/// 4. Array values in nested sections are preserved
///
/// # Test Setup
/// Creates TOML files with:
/// - Multiple levels of nested tool sections
/// - Various value types in nested sections
/// - Poetry section for exclusion
///
/// # Verification Steps
/// 1. Verifies nested section presence
/// 2. Confirms nested values are preserved
/// 3. Validates array value preservation
/// 4. Checks section hierarchy integrity
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

    assert!(
        !result.matches("[tool]").any(|_| true),
        "Should not have empty [tool] section"
    );
    let tool_sections = result.matches("[tool.").count();
    assert!(tool_sections > 0, "Should have non-empty tool sections");
}

/// Test handling of empty nested sections.
///
/// This test verifies that:
/// 1. Empty sections are properly cleaned up
/// 2. Non-empty sections are preserved
/// 3. Empty nested sections are removed
/// 4. Section hierarchy remains intact for non-empty sections
///
/// # Test Setup
/// Creates TOML files with:
/// - Mix of empty and non-empty sections
/// - Empty nested sections
/// - Valid sections with content
///
/// # Verification Steps
/// 1. Confirms empty sections are removed
/// 2. Verifies non-empty sections remain
/// 3. Checks nested section cleanup
/// 4. Validates overall structure integrity
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

    assert!(
        !result.matches("[tool]").any(|_| true),
        "Should not have empty [tool] section"
    );
}

/// Test handling of empty tool sections.
///
/// This test verifies that:
/// 1. Empty tool sections are not created
/// 2. Project content remains unchanged
/// 3. No unnecessary sections are added
///
/// # Test Setup
/// Creates TOML files with:
/// - Only poetry section in old file
/// - Basic project configuration in new file
///
/// # Verification Steps
/// 1. Verifies no empty tool section is created
/// 2. Confirms project content is preserved
/// 3. Validates overall file structure
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

/// Test cleanup of empty tool sections after processing.
///
/// This test verifies that:
/// 1. Empty tool sections are removed during cleanup
/// 2. Empty tool subsections are removed
/// 3. Project content remains intact
/// 4. Overall file structure is maintained
///
/// # Test Setup
/// Creates TOML files with:
/// - Empty tool sections
/// - Poetry configuration
/// - Empty black configuration
///
/// # Verification Steps
/// 1. Confirms empty tool sections are removed
/// 2. Verifies empty black section is cleaned up
/// 3. Validates project content preservation
/// 4. Checks final file structure
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

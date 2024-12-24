use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use uv_migrator::utils::pyproject::append_tool_sections;

#[cfg(test)]
mod tests {
    use super::*;

    const LINE_LENGTH: &str = "88";
    const PY_VERSION: &str = "py39";

    struct TestContext {
        temp_dir: TempDir,
        project_dir: PathBuf,
    }

    impl TestContext {
        fn new() -> io::Result<Self> {
            let temp_dir = tempfile::tempdir()?;
            let project_dir = temp_dir.path().to_path_buf();
            Ok(Self {
                temp_dir,
                project_dir,
            })
        }

        fn create_file(&self, name: &str, content: &str) -> io::Result<PathBuf> {
            let path = self.project_dir.join(name);
            let mut file = fs::File::create(&path)?;
            file.write_all(content.as_bytes())?;
            Ok(path)
        }

        fn path(&self) -> &Path {
            &self.project_dir
        }
    }

    /// Test appending tool sections to an empty project.
    ///
    /// This test verifies that:
    /// 1. Existing tool sections from `old.pyproject.toml` are correctly appended to `pyproject.toml`
    /// 2. The updated `pyproject.toml` contains the expected tool sections and configurations
    /// 3. No empty sections like `[tool]` are present in the updated `pyproject.toml`
    ///
    /// # Returns
    ///
    /// An `io::Result` indicating whether the test passed or failed with an error message
    #[test]
    fn test_append_tool_sections_with_empty_project() -> io::Result<()> {
        let ctx = TestContext::new()?;

        // Create old.pyproject.toml
        let old_pyproject_content = format!(
            r#"
            [tool.black]
            line-length = {LINE_LENGTH}

            [tool.isort]
            profile = "black"
        "#
        );
        ctx.create_file("old.pyproject.toml", &old_pyproject_content)?;

        // Create pyproject.toml
        let pyproject_content = r#"
            [tool.poetry]
            name = "example"
            version = "0.1.0"
        "#;
        let pyproject_path = ctx.create_file("pyproject.toml", pyproject_content)?;

        // Call the function
        append_tool_sections(ctx.path()).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Read and verify the updated pyproject.toml
        let updated_content = fs::read_to_string(&pyproject_path)?;

        assert!(
            updated_content.contains("[tool.poetry]"),
            "Missing poetry section"
        );
        for (section, config) in [
            ("black", format!("line-length = {LINE_LENGTH}")),
            ("isort", "profile = \"black\"".to_owned()),
        ] {
            assert!(
                updated_content.contains(&format!("[tool.{}]", section)),
                "Missing {} section",
                section
            );
            assert!(
                updated_content.contains(&config),
                "Missing {} configuration",
                section
            );
        }

        Ok(())
    }

    /// Test appending tool sections to a project.
    ///
    /// This test verifies that:
    /// 1. Existing tool sections from `old.pyproject.toml` are correctly appended to `pyproject.toml`
    /// 2. The updated `pyproject.toml` contains the expected tool sections and configurations
    /// 3. No empty sections like `[tool]` are present in the updated `pyproject.toml`
    ///
    /// # Returns
    ///
    /// An `io::Result` indicating whether the test passed or failed with an error message
    #[test]
    fn test_append_tool_sections() -> io::Result<()> {
        let ctx = TestContext::new()?;

        let old_pyproject_content = format!(
            r#"
        [tool.black]
        line-length = {LINE_LENGTH}
        target-version = ["{PY_VERSION}"]

        [tool.isort]
        profile = "black"
        line_length = {LINE_LENGTH}

        [tool.ruff]
        target-version = "{PY_VERSION}"
        line-length = {LINE_LENGTH}

        [tool.mypy]
        ignore_missing_imports = true
        namespace_packages = false

        [tool]
    "#
        );
        ctx.create_file("old.pyproject.toml", &old_pyproject_content)?;

        let pyproject_content = r#"
        [build-system]
        requires = ["hatchling"]
        build-backend = "hatchling.build"
    "#;
        let pyproject_path = ctx.create_file("pyproject.toml", pyproject_content)?;

        append_tool_sections(ctx.path()).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let updated_content = fs::read_to_string(&pyproject_path)?;

        // Verify all sections with detailed messages
        let expected_sections = [
            (
                "black",
                vec![
                    format!("line-length = {LINE_LENGTH}"),
                    format!("target-version = [\"{PY_VERSION}\"]"),
                ],
            ),
            (
                "isort",
                vec![
                    "profile = \"black\"".to_string(),
                    format!("line_length = {LINE_LENGTH}"),
                ],
            ),
            (
                "ruff",
                vec![
                    format!("target-version = \"{PY_VERSION}\""),
                    format!("line-length = {LINE_LENGTH}"),
                ],
            ),
            (
                "mypy",
                vec![
                    "ignore_missing_imports = true".to_string(),
                    "namespace_packages = false".to_string(),
                ],
            ),
        ];

        for (section, configs) in expected_sections {
            assert!(
                updated_content.contains(&format!("[tool.{}]", section)),
                "Missing {} section",
                section
            );
            for config in configs {
                assert!(
                    updated_content.contains(&config),
                    "Missing {} config: {}",
                    section,
                    config
                );
            }
        }

        // Negative case: Ensure no empty sections like [tool] are present
        assert!(
            !updated_content.contains("[tool]\n"),
            "Empty [tool] section should not be present"
        );

        Ok(())
    }
}

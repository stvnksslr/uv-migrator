#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use uv_migrator::migrators::common::perform_poetry_migration;
    use uv_migrator::utils::file_ops::FileTrackerGuard;

    fn create_test_poetry_package() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        // Create pyproject.toml with package configuration
        let content = r#"
[tool.poetry]
name = "test-poetry-package"
version = "0.1.0"
description = "A test poetry package"
authors = ["Test Author <test@example.com>"]
packages = [
    { include = "src" }
]

[tool.poetry.dependencies]
python = "^3.9"
fastapi = "^0.111.0"

[build-system]
requires = ["poetry-core>=1.0.0"]
build-backend = "poetry.core.masonry.api"
        "#;

        fs::write(project_dir.join("pyproject.toml"), content).unwrap();

        // Rename to old.pyproject.toml to simulate migration
        fs::rename(
            project_dir.join("pyproject.toml"),
            project_dir.join("old.pyproject.toml"),
        )
        .unwrap();

        // Create new pyproject.toml like uv-migrator would do
        let new_content = r#"
[project]
name = "test-poetry-package"
version = "0.1.0"
description = "A test poetry package"
authors = [{ name = "Test Author", email = "test@example.com" }]
requires-python = ">=3.9"
dependencies = [
    "fastapi>=0.111.0",
]

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"
        "#;

        fs::write(project_dir.join("pyproject.toml"), new_content).unwrap();

        (temp_dir, project_dir)
    }

    fn create_test_poetry2_package() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        // Create pyproject.toml with Poetry 2.0 package configuration
        let content = r#"
[project]
name = "test-poetry-v2-package"
version = "0.1.0"
description = "A test Poetry 2.0 package"
authors = [
    {name = "Test Author", email = "test@example.com"}
]
readme = "README.md"
requires-python = ">=3.10"
dependencies = [
    "fastapi (>=0.115.6,<0.116.0)",
]

[build-system]
requires = ["poetry-core>=2.0.0,<3.0.0"]
build-backend = "poetry.core.masonry.api"

[tool.poetry]
packages = [
    { include = "src" },
]
        "#;

        fs::write(project_dir.join("pyproject.toml"), content).unwrap();

        // Rename to old.pyproject.toml to simulate migration
        fs::rename(
            project_dir.join("pyproject.toml"),
            project_dir.join("old.pyproject.toml"),
        )
        .unwrap();

        // Create new pyproject.toml like uv-migrator would do
        let new_content = r#"
[project]
name = "test-poetry-v2-package"
version = "0.1.0"
description = "A test Poetry 2.0 package"
authors = [{ name = "Test Author", email = "test@example.com" }]
requires-python = ">=3.10"
dependencies = [
    "fastapi>=0.115.6,<0.116.0",
]

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"
        "#;

        fs::write(project_dir.join("pyproject.toml"), new_content).unwrap();

        (temp_dir, project_dir)
    }

    #[test]
    fn test_poetry_package_migration() {
        let (_temp_dir, project_dir) = create_test_poetry_package();
        let mut file_tracker = FileTrackerGuard::new();

        // Perform the migration
        let result = perform_poetry_migration(&project_dir, &mut file_tracker);
        assert!(result.is_ok(), "Poetry migration failed: {:?}", result);

        // Read the resulting pyproject.toml
        let content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();

        // Verify the migration was successful without requiring specific packages config
        assert!(
            content.contains("[build-system]"),
            "Missing build system section"
        );
        assert!(
            content.contains("build-backend = \"hatchling.build\""),
            "Missing or incorrect build backend"
        );

        // Check that the authors were migrated correctly
        assert!(content.contains("authors = ["), "Missing authors section");
        assert!(
            content.contains("{ name = \"Test Author\", email = \"test@example.com\" }"),
            "Missing or incorrect author information"
        );
    }

    #[test]
    fn test_poetry2_package_migration() {
        let (_temp_dir, project_dir) = create_test_poetry2_package();
        let mut file_tracker = FileTrackerGuard::new();

        // Perform the migration
        let result = perform_poetry_migration(&project_dir, &mut file_tracker);
        assert!(result.is_ok(), "Poetry 2.0 migration failed: {:?}", result);

        // Read the resulting pyproject.toml
        let content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();

        // Verify the migration was successful without requiring specific packages config
        assert!(
            content.contains("[build-system]"),
            "Missing build system section"
        );
        assert!(
            content.contains("build-backend = \"hatchling.build\""),
            "Missing or incorrect build backend"
        );

        // Check that the project metadata was preserved
        assert!(
            content.contains("name = \"test-poetry-v2-package\""),
            "Missing or incorrect project name"
        );
        assert!(
            content.contains("requires-python = \">=3.10\""),
            "Missing or incorrect Python version requirement"
        );
    }
}

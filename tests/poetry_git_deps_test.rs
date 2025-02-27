#[cfg(test)]
mod git_dependency_tests {
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use uv_migrator::migrators::common::perform_poetry_migration;
    use uv_migrator::utils::file_ops::FileTrackerGuard;

    fn create_test_poetry_project_with_git_deps() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        // Create pyproject.toml with git dependency
        let content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"
description = "A test project with git dependencies"
authors = ["Test Author <test@example.com>"]

[tool.poetry.dependencies]
python = "^3.9"
requests = "^2.28.0"
dependency = { git = "https://github.com/user/library.git", branch = "my-branch" }
another-dep = { git = "https://github.com/user/another-lib.git", tag = "v1.0.0" }
revision-dep = { git = "https://github.com/user/rev-lib.git", rev = "123abc" }

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
name = "test-project"
version = "0.1.0"
description = "A test project with git dependencies"
authors = [{ name = "Test Author", email = "test@example.com" }]
requires-python = ">=3.9"
dependencies = [
    "requests>=2.28.0",
    "dependency>=1.0.0",
    "another-dep>=1.0.0",
    "revision-dep>=1.0.0",
]

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"
        "#;

        fs::write(project_dir.join("pyproject.toml"), new_content).unwrap();

        (temp_dir, project_dir)
    }

    #[test]
    fn test_git_dependency_migration() {
        let (_temp_dir, project_dir) = create_test_poetry_project_with_git_deps();
        let mut file_tracker = FileTrackerGuard::new();

        // Perform the migration
        let result = perform_poetry_migration(&project_dir, &mut file_tracker);
        assert!(result.is_ok(), "Poetry migration failed: {:?}", result);

        // Read the resulting pyproject.toml
        let content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();

        // Verify the git dependencies were migrated correctly
        assert!(
            content.contains("[tool.uv.sources]")
                || content.contains("[tool.uv.sources.dependency]"),
            "Missing [tool.uv.sources] section"
        );

        // Check for first git dependency with branch
        assert!(
            content.contains(r#"[tool.uv.sources.dependency]"#),
            "Missing dependency in sources"
        );
        assert!(
            content.contains(r#"git = "https://github.com/user/library.git""#),
            "Missing git URL for dependency"
        );
        assert!(
            content.contains(r#"branch = "my-branch""#),
            "Missing branch for dependency"
        );

        // Check for second git dependency with tag
        assert!(
            content.contains(r#"[tool.uv.sources.another-dep]"#),
            "Missing another-dep in sources"
        );
        assert!(
            content.contains(r#"git = "https://github.com/user/another-lib.git""#),
            "Missing git URL for another-dep"
        );
        assert!(
            content.contains(r#"tag = "v1.0.0""#),
            "Missing tag for another-dep"
        );

        // Check for third git dependency with revision
        assert!(
            content.contains(r#"[tool.uv.sources.revision-dep]"#),
            "Missing revision-dep in sources"
        );
        assert!(
            content.contains(r#"git = "https://github.com/user/rev-lib.git""#),
            "Missing git URL for revision-dep"
        );
        assert!(
            content.contains(r#"rev = "123abc""#),
            "Missing revision for revision-dep"
        );
    }
}

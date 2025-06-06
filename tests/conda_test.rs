use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use uv_migrator::DependencyType;
use uv_migrator::migrators::MigrationSource;
use uv_migrator::migrators::conda::CondaMigrationSource;

/// Helper function to create a temporary test project with an environment.yml file.
///
/// # Arguments
///
/// * `content` - The content to write to the environment.yml file
///
/// # Returns
///
/// A tuple containing the temporary directory and its path
fn create_test_environment(content: &str) -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().to_path_buf();

    fs::write(project_dir.join("environment.yml"), content).unwrap();

    (temp_dir, project_dir)
}

/// Test detection of Conda projects
#[test]
fn test_detect_conda_project() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().to_path_buf();

    // Test with environment.yml
    fs::write(project_dir.join("environment.yml"), "").unwrap();
    assert!(CondaMigrationSource::detect_project_type(&project_dir));

    // Clean up and test with environment.yaml
    fs::remove_file(project_dir.join("environment.yml")).unwrap();
    fs::write(project_dir.join("environment.yaml"), "").unwrap();
    assert!(CondaMigrationSource::detect_project_type(&project_dir));

    // Test without environment file
    fs::remove_file(project_dir.join("environment.yaml")).unwrap();
    assert!(!CondaMigrationSource::detect_project_type(&project_dir));
}

/// Test extraction of basic Conda dependencies
#[test]
fn test_extract_basic_conda_dependencies() {
    let content = r#"
name: test-env
channels:
  - conda-forge
  - defaults
dependencies:
  - python=3.9
  - numpy=1.21.5
  - pandas>=1.3.0
  - scikit-learn
  - matplotlib
"#;

    let (_temp_dir, project_dir) = create_test_environment(content);
    let source = CondaMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    // Should skip python
    assert_eq!(dependencies.len(), 4);

    // Check numpy
    let numpy_dep = dependencies.iter().find(|d| d.name == "numpy").unwrap();
    assert_eq!(numpy_dep.version, Some("==1.21.5".to_string()));
    assert_eq!(numpy_dep.dep_type, DependencyType::Main);

    // Check pandas
    let pandas_dep = dependencies.iter().find(|d| d.name == "pandas").unwrap();
    assert_eq!(pandas_dep.version, Some(">=1.3.0".to_string()));

    // Check scikit-learn (no version)
    let sklearn_dep = dependencies
        .iter()
        .find(|d| d.name == "scikit-learn")
        .unwrap();
    assert_eq!(sklearn_dep.version, None);
}

/// Test extraction of dependencies with wildcards
#[test]
fn test_extract_wildcard_dependencies() {
    let content = r#"
name: test-env
dependencies:
  - numpy=1.21.*
  - pandas=1.*
  - scipy=*
"#;

    let (_temp_dir, project_dir) = create_test_environment(content);
    let source = CondaMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    assert_eq!(dependencies.len(), 3);

    // Check numpy with minor wildcard
    let numpy_dep = dependencies.iter().find(|d| d.name == "numpy").unwrap();
    assert_eq!(numpy_dep.version, Some(">=1.21.0,<1.22.0".to_string()));

    // Check pandas with major wildcard
    let pandas_dep = dependencies.iter().find(|d| d.name == "pandas").unwrap();
    assert_eq!(pandas_dep.version, Some(">=1.0.0,<2.0.0".to_string()));

    // Check scipy with any version
    let scipy_dep = dependencies.iter().find(|d| d.name == "scipy").unwrap();
    assert_eq!(scipy_dep.version, None);
}

/// Test extraction of pip dependencies within Conda environment
#[test]
fn test_extract_pip_dependencies() {
    let content = r#"
name: test-env
channels:
  - conda-forge
dependencies:
  - python=3.9
  - numpy
  - pip
  - pip:
    - requests==2.28.0
    - flask>=2.0.0
    - django[rest]>=4.0.0
    - beautifulsoup4
"#;

    let (_temp_dir, project_dir) = create_test_environment(content);
    let source = CondaMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    // Should have 1 conda dep (numpy) + 4 pip deps
    assert_eq!(dependencies.len(), 5);

    // Check requests
    let requests_dep = dependencies.iter().find(|d| d.name == "requests").unwrap();
    assert_eq!(requests_dep.version, Some("==2.28.0".to_string()));

    // Check flask
    let flask_dep = dependencies.iter().find(|d| d.name == "flask").unwrap();
    assert_eq!(flask_dep.version, Some(">=2.0.0".to_string()));

    // Check django with extras
    let django_dep = dependencies.iter().find(|d| d.name == "django").unwrap();
    assert_eq!(django_dep.version, Some(">=4.0.0".to_string()));
    assert_eq!(django_dep.extras, Some(vec!["rest".to_string()]));
}

/// Test package name mapping from Conda to PyPI
#[test]
fn test_conda_to_pypi_mapping() {
    let content = r#"
name: ml-env
dependencies:
  - pytorch
  - tensorflow-gpu
  - py-opencv
  - pillow-simd
  - ruamel_yaml
  - importlib_metadata
  - prompt_toolkit
"#;

    let (_temp_dir, project_dir) = create_test_environment(content);
    let source = CondaMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    // Check pytorch -> torch
    assert!(dependencies.iter().any(|d| d.name == "torch"));
    assert!(!dependencies.iter().any(|d| d.name == "pytorch"));

    // Check tensorflow-gpu -> tensorflow
    assert!(dependencies.iter().any(|d| d.name == "tensorflow"));
    assert!(!dependencies.iter().any(|d| d.name == "tensorflow-gpu"));

    // Check py-opencv -> opencv-python
    assert!(dependencies.iter().any(|d| d.name == "opencv-python"));
    assert!(!dependencies.iter().any(|d| d.name == "py-opencv"));

    // Check pillow-simd -> pillow
    assert!(dependencies.iter().any(|d| d.name == "pillow"));
    assert!(!dependencies.iter().any(|d| d.name == "pillow-simd"));

    // Check ruamel_yaml -> ruamel.yaml
    assert!(dependencies.iter().any(|d| d.name == "ruamel.yaml"));
    assert!(!dependencies.iter().any(|d| d.name == "ruamel_yaml"));

    // Check importlib_metadata -> importlib-metadata
    assert!(dependencies.iter().any(|d| d.name == "importlib-metadata"));
    assert!(!dependencies.iter().any(|d| d.name == "importlib_metadata"));

    // Check prompt_toolkit -> prompt-toolkit
    assert!(dependencies.iter().any(|d| d.name == "prompt-toolkit"));
    assert!(!dependencies.iter().any(|d| d.name == "prompt_toolkit"));
}

/// Test that system packages are skipped
#[test]
fn test_skip_system_packages() {
    let content = r#"
name: test-env
dependencies:
  - python=3.9
  - numpy
  - libgcc-ng
  - openssl
  - mkl
  - cudatoolkit
  - gcc
  - make
"#;

    let (_temp_dir, project_dir) = create_test_environment(content);
    let source = CondaMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    // Should only have numpy
    assert_eq!(dependencies.len(), 1);
    assert_eq!(dependencies[0].name, "numpy");
}

/// Test that packages starting with underscore are skipped
#[test]
fn test_skip_underscore_packages() {
    let content = r#"
name: test-env
dependencies:
  - python=3.9
  - numpy
  - _libgcc_mutex
  - _openmp_mutex
  - _pytorch_select
  - _low_priority
  - pandas
"#;

    let (_temp_dir, project_dir) = create_test_environment(content);
    let source = CondaMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    // Should only have numpy and pandas (not the underscore packages)
    assert_eq!(dependencies.len(), 2);
    assert!(dependencies.iter().any(|d| d.name == "numpy"));
    assert!(dependencies.iter().any(|d| d.name == "pandas"));

    // Verify no underscore packages were included
    assert!(!dependencies.iter().any(|d| d.name.starts_with('_')));
}

/// Test that conda-specific packages are skipped
#[test]
fn test_skip_conda_specific_packages() {
    let content = r#"
name: test-env
dependencies:
  - python=3.9
  - numpy
  - anaconda=2020.07
  - anaconda-client
  - anaconda-navigator
  - navigator-updater
  - dask-core
  - matplotlib-base
  - numpy-base
  - backports=1.0
  - pandas
"#;

    let (_temp_dir, project_dir) = create_test_environment(content);
    let source = CondaMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    // Should only have numpy and pandas
    assert_eq!(dependencies.len(), 2);
    assert!(dependencies.iter().any(|d| d.name == "numpy"));
    assert!(dependencies.iter().any(|d| d.name == "pandas"));

    // Verify conda-specific packages were not included
    assert!(!dependencies.iter().any(|d| d.name == "anaconda"));
    assert!(!dependencies.iter().any(|d| d.name == "anaconda-client"));
    assert!(!dependencies.iter().any(|d| d.name == "dask-core"));
    assert!(!dependencies.iter().any(|d| d.name == "matplotlib-base"));
    assert!(!dependencies.iter().any(|d| d.name == "backports"));
}

/// Test extraction of Python version from environment
#[test]
fn test_extract_python_version() {
    let content = r#"
name: test-env
dependencies:
  - python=3.9.7
  - numpy
"#;

    let (_temp_dir, project_dir) = create_test_environment(content);
    let python_version =
        CondaMigrationSource::extract_python_version_from_environment(&project_dir).unwrap();

    assert_eq!(python_version, Some("3.9".to_string()));
}

/// Test complex pip dependencies with extras and markers
#[test]
fn test_complex_pip_dependencies() {
    let content = r#"
name: test-env
dependencies:
  - pip:
    - "apache-airflow[postgres,google]==2.7.0"
    - "pytest[testing]>=7.0.0"
    - "torch[cpu]>=2.0.0,<3.0.0"
"#;

    let (_temp_dir, project_dir) = create_test_environment(content);
    let source = CondaMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    // Check apache-airflow with multiple extras
    let airflow_dep = dependencies
        .iter()
        .find(|d| d.name == "apache-airflow")
        .unwrap();
    assert_eq!(airflow_dep.version, Some("==2.7.0".to_string()));
    assert_eq!(
        airflow_dep.extras,
        Some(vec!["postgres".to_string(), "google".to_string()])
    );

    // Check torch with version range
    let torch_dep = dependencies.iter().find(|d| d.name == "torch").unwrap();
    assert_eq!(torch_dep.version, Some(">=2.0.0,<3.0.0".to_string()));
    assert_eq!(torch_dep.extras, Some(vec!["cpu".to_string()]));
}

/// Test handling of missing or empty environment file
#[test]
fn test_empty_environment() {
    let content = r#"
name: empty-env
"#;

    let (_temp_dir, project_dir) = create_test_environment(content);
    let source = CondaMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    assert!(dependencies.is_empty());
}

/// Test handling of environment file without dependencies section
#[test]
fn test_no_dependencies_section() {
    let content = r#"
name: test-env
channels:
  - conda-forge
  - defaults
"#;

    let (_temp_dir, project_dir) = create_test_environment(content);
    let source = CondaMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    assert!(dependencies.is_empty());
}

/// Test handling of packages with dots in their names
#[test]
fn test_packages_with_dots() {
    let content = r#"
name: test-env
dependencies:
  - backports.functools_lru_cache=1.6.1
  - backports.shutil_get_terminal_size=1.0.0
  - zope.event=4.4
  - zope.interface=4.7.1
"#;

    let (_temp_dir, project_dir) = create_test_environment(content);
    let source = CondaMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    assert_eq!(dependencies.len(), 4);

    // Check backports.functools_lru_cache
    let backports_dep = dependencies
        .iter()
        .find(|d| d.name == "backports.functools_lru_cache")
        .unwrap();
    assert_eq!(backports_dep.version, Some("==1.6.1".to_string()));

    // Check zope.event
    let zope_event_dep = dependencies
        .iter()
        .find(|d| d.name == "zope.event")
        .unwrap();
    assert_eq!(zope_event_dep.version, Some("==4.4".to_string()));
}

/// Test that backports namespace is skipped but individual backports packages are kept
#[test]
fn test_backports_namespace_handling() {
    let content = r#"
name: test-env
dependencies:
  - backports=1.0
  - backports.functools_lru_cache=1.6.1
  - backports.shutil_get_terminal_size=1.0.0
  - backports.tempfile=1.0
  - backports.weakref=1.0.post1
"#;

    let (_temp_dir, project_dir) = create_test_environment(content);
    let source = CondaMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    // Should have 4 dependencies (all backports.* except the namespace package)
    assert_eq!(dependencies.len(), 4);

    // Verify backports namespace package was skipped
    assert!(!dependencies.iter().any(|d| d.name == "backports"));

    // Verify individual backports packages were included
    assert!(
        dependencies
            .iter()
            .any(|d| d.name == "backports.functools_lru_cache")
    );
    assert!(
        dependencies
            .iter()
            .any(|d| d.name == "backports.shutil_get_terminal_size")
    );
    assert!(dependencies.iter().any(|d| d.name == "backports.tempfile"));
    assert!(dependencies.iter().any(|d| d.name == "backports.weakref"));
}

/// Test that problematic package versions are updated
#[test]
fn test_problematic_version_updates() {
    let content = r#"
name: test-env
dependencies:
  - bokeh=2.1.1
  - numpy=1.18.5
"#;

    let (_temp_dir, project_dir) = create_test_environment(content);
    let source = CondaMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    // Check that bokeh version was updated
    let bokeh_dep = dependencies.iter().find(|d| d.name == "bokeh").unwrap();
    assert_eq!(bokeh_dep.version, Some("==2.4.3".to_string()));

    // Check that numpy version was not changed
    let numpy_dep = dependencies.iter().find(|d| d.name == "numpy").unwrap();
    assert_eq!(numpy_dep.version, Some("==1.18.5".to_string()));
}

use uv_migrator::migrators;
use uv_migrator::models::{Dependency, DependencyType};

/// Test formatting of dependencies for UV CLI use.
///
/// This test verifies that the format_dependency function correctly formats:
/// 1. Simple dependencies with version
/// 2. Dependencies with extras
/// 3. Dependencies with multiple extras
/// 4. Dependencies with environment markers
#[test]
fn test_format_dependency() {
    // Use the public format_dependency function directly
    let format_dependency = migrators::format_dependency;

    // Test simple dependency with version
    let dep1 = Dependency {
        name: "requests".to_string(),
        version: Some("2.28.1".to_string()),
        dep_type: DependencyType::Main,
        environment_markers: None,
        extras: None,
    };
    assert_eq!(format_dependency(&dep1), "requests==2.28.1");

    // Test dependency with extras
    let dep2 = Dependency {
        name: "uvicorn".to_string(),
        version: Some("^0.30.1".to_string()),
        dep_type: DependencyType::Main,
        environment_markers: None,
        extras: Some(vec!["standard".to_string()]),
    };
    assert_eq!(format_dependency(&dep2), "uvicorn[standard]>=0.30.1");

    // Test dependency with multiple extras
    let dep3 = Dependency {
        name: "ibis-framework".to_string(),
        version: Some("^10.0.0".to_string()),
        dep_type: DependencyType::Main,
        environment_markers: None,
        extras: Some(vec![
            "bigquery".to_string(),
            "duckdb".to_string(),
            "polars".to_string(),
        ]),
    };
    assert_eq!(
        format_dependency(&dep3),
        "ibis-framework[bigquery,duckdb,polars]>=10.0.0"
    );

    // Test dependency with environment markers
    let dep4 = Dependency {
        name: "dataclasses".to_string(),
        version: Some("1.0.0".to_string()),
        dep_type: DependencyType::Main,
        environment_markers: Some("python_version < '3.7'".to_string()),
        extras: None,
    };
    assert_eq!(
        format_dependency(&dep4),
        "dataclasses==1.0.0; python_version < '3.7'"
    );

    // Test dependency with both extras and environment markers
    let dep5 = Dependency {
        name: "django".to_string(),
        version: Some("~=4.2.0".to_string()),
        dep_type: DependencyType::Main,
        environment_markers: Some("platform_system != 'Windows'".to_string()),
        extras: Some(vec!["rest".to_string(), "admin".to_string()]),
    };
    assert_eq!(
        format_dependency(&dep5),
        "django[rest,admin]~=4.2.0; platform_system != 'Windows'"
    );
}

use log::{debug, error, info};
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::{exit, Command};
use toml;
use which::which;

#[derive(Deserialize, Debug)]
struct PyProject {
    project: Option<Project>,
    tool: Option<Tool>,
}

#[derive(Deserialize, Debug)]
struct Project {
    dependencies: Option<HashMap<String, String>>,
    optional_dependencies: Option<HashMap<String, HashMap<String, String>>>,
}

#[derive(Deserialize, Debug)]
struct Tool {
    poetry: Option<Poetry>,
}

#[derive(Deserialize, Debug)]
struct Poetry {
    dependencies: Option<HashMap<String, toml::Value>>,
    group: Option<HashMap<String, Group>>,
}

#[derive(Deserialize, Debug)]
struct Group {
    dependencies: HashMap<String, toml::Value>,
}

fn create_virtual_environment() -> Result<(), String> {
    info!("create_virtual_environment: Creating a new virtual environment");
    let uv_path = which("uv").map_err(|e| format!("Failed to find uv command: {}", e))?;
    let output = Command::new(uv_path).arg("venv").output().map_err(|e| {
        format!(
            "create_virtual_environment: Failed to execute uv venv: {}",
            e
        )
    })?;

    if output.status.success() {
        info!("create_virtual_environment: Virtual environment created successfully");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "create_virtual_environment: Failed to create virtual environment: {}",
            stderr
        ))
    }
}

fn format_dependency(name: &str, value: &toml::Value) -> String {
    match value {
        toml::Value::String(v) => format!("{}=={}", name, v.trim_start_matches('^')),
        toml::Value::Table(t) => {
            if let Some(toml::Value::String(version)) = t.get("version") {
                format!("{}=={}", name, version.trim_start_matches('^'))
            } else {
                name.to_string()
            }
        }
        _ => name.to_string(),
    }
}

fn should_include_dependency(dep: &str, formatted_dep: &str) -> bool {
    !(dep == "python" || formatted_dep.starts_with("python=="))
}

fn rename_pyproject(pyproject_path: &Path) -> Result<(), String> {
    let old_pyproject_path = pyproject_path.with_file_name("old.pyproject.toml");

    if let Err(e) = fs::rename(pyproject_path, &old_pyproject_path) {
        return Err(format!(
            "rename_pyproject: Failed to rename pyproject.toml: {}",
            e
        ));
    }
    info!("rename_pyproject: Renamed existing pyproject.toml to old.pyproject.toml");
    Ok(())
}

fn create_new_pyproject(project_dir: &Path) -> Result<(), String> {
    info!("create_new_pyproject: Initializing new project with uv init");

    let original_dir =
        env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;
    debug!("Original directory: {:?}", original_dir);

    let target_dir = if project_dir.as_os_str().is_empty() {
        &original_dir
    } else {
        project_dir
    };
    debug!("Target directory: {:?}", target_dir);

    env::set_current_dir(target_dir)
        .map_err(|e| format!("Failed to change directory to {:?}: {}", target_dir, e))?;
    debug!("Changed to directory: {:?}", env::current_dir().unwrap());

    let uv_path = which("uv").map_err(|e| format!("Failed to find uv command: {}", e))?;
    debug!("UV command path: {:?}", uv_path);

    let full_command = format!("{} init", uv_path.display());
    info!("create_new_pyproject: Executing command: {}", full_command);

    let output = Command::new(&uv_path)
        .arg("init")
        .arg("--no-pin-python")
        .output()
        .map_err(|e| format!("create_new_pyproject: Failed to execute uv init: {}", e))?;

    if output.status.success() {
        info!("create_new_pyproject: Successfully initialized new project with uv init");

        // Delete hello.py if it exists
        let hello_py_path = target_dir.join("hello.py");
        if hello_py_path.exists() {
            fs::remove_file(&hello_py_path)
                .map_err(|e| format!("Failed to delete hello.py: {}", e))?;
            info!("create_new_pyproject: Deleted hello.py file");
        } else {
            info!("create_new_pyproject: hello.py file not found, skipping deletion");
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("create_new_pyproject: uv init failed: {}", stderr));
    }

    // Change back to the original directory
    env::set_current_dir(&original_dir)
        .map_err(|e| format!("Failed to change back to original directory: {}", e))?;

    Ok(())
}


fn add_dependencies(deps: &[String], dev: bool) -> Result<(), String> {
    if deps.is_empty() {
        info!(
            "add_dependencies: No {} dependencies to add.",
            if dev { "dev" } else { "main" }
        );
        return Ok(());
    }

    let uv_path = which("uv").map_err(|e| format!("Failed to find uv command: {}", e))?;
    let mut command = Command::new(uv_path);
    command.arg("add");
    if dev {
        command.arg("--dev");
    }
    command.args(deps);

    let full_command = format!("{:?}", command);
    info!("add_dependencies: Executing command: {}", full_command);

    let output = command
        .output()
        .map_err(|e| format!("add_dependencies: Failed to execute uv command: {}", e))?;

    if output.status.success() {
        info!(
            "add_dependencies: {} dependencies added successfully!",
            if dev { "Dev" } else { "Main" }
        );
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "add_dependencies: Failed to add {} dependencies: {}",
            if dev { "dev" } else { "main" },
            stderr
        ))
    }
}

fn add_all_dependencies(pyproject: &PyProject) -> Result<(), String> {
    let mut main_deps = Vec::new();
    let mut dev_deps = Vec::new();

    if let Some(project) = &pyproject.project {
        // Handle PEP 621 format
        info!("add_all_dependencies: Found [project] section (PEP 621 format)");
        if let Some(deps) = &project.dependencies {
            info!("add_all_dependencies: Found main dependencies in [project.dependencies]");
            main_deps.extend(deps.iter().map(|(k, v)| format!("{}=={}", k, v)));
        }
        if let Some(optional_deps) = &project.optional_dependencies {
            info!("add_all_dependencies: Found optional dependencies in [project.optional-dependencies]");
            for (_, group_deps) in optional_deps {
                dev_deps.extend(group_deps.iter().map(|(k, v)| format!("{}=={}", k, v)));
            }
        }
    } else if let Some(tool) = &pyproject.tool {
        if let Some(poetry) = &tool.poetry {
            // Handle Poetry format
            info!("add_all_dependencies: Found [tool.poetry] section");
            if let Some(deps) = &poetry.dependencies {
                info!(
                    "add_all_dependencies: Found main dependencies in [tool.poetry.dependencies]"
                );
                main_deps.extend(deps.iter().filter_map(|(dep, value)| {
                    let formatted = format_dependency(dep, value);
                    if should_include_dependency(dep, &formatted) {
                        Some(formatted)
                    } else {
                        None
                    }
                }));
            }

            if let Some(groups) = &poetry.group {
                info!("add_all_dependencies: Found dependency groups in [tool.poetry.group]");
                for (group_name, group) in groups {
                    info!("add_all_dependencies: Processing group: {}", group_name);
                    dev_deps.extend(group.dependencies.iter().filter_map(|(dep, value)| {
                        let formatted = format_dependency(dep, value);
                        if should_include_dependency(dep, &formatted) {
                            Some(formatted)
                        } else {
                            None
                        }
                    }));
                }
            }
        } else {
            return Err(
                "add_all_dependencies: No [tool.poetry] section found in pyproject.toml"
                    .to_string(),
            );
        }
    } else {
        return Err("add_all_dependencies: Neither [project] (PEP 621) nor [tool.poetry] section found in pyproject.toml".to_string());
    }

    debug!("add_all_dependencies: Main dependencies: {:?}", main_deps);
    debug!("add_all_dependencies: Dev dependencies: {:?}", dev_deps);

    // Remove duplicates
    main_deps.sort();
    main_deps.dedup();
    dev_deps.sort();
    dev_deps.dedup();

    // Add main dependencies
    add_dependencies(&main_deps, false)?;

    // Add dev dependencies
    add_dependencies(&dev_deps, true)?;

    Ok(())
}

fn main() {
    env_logger::init();

    if let Err(e) = which("uv") {
        error!("main: The 'uv' command is not available. Please install uv and ensure it's in your PATH. Error: {}", e);
        exit(1);
    }

    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        error!("main: Usage: {} <path_to_pyproject.toml>", args[0]);
        exit(1);
    }

    let file_path = Path::new(&args[1]);
    let project_dir = file_path.parent().unwrap_or(Path::new(""));

    info!(
        "main: Current working directory: {:?}",
        env::current_dir().unwrap()
    );
    info!("main: Project directory: {:?}", project_dir);

    if !file_path.exists() {
        error!(
            "main: The specified pyproject.toml file does not exist: {}",
            file_path.display()
        );
        exit(1);
    }

    if !file_path.is_file() {
        error!(
            "main: The specified path is not a file: {}",
            file_path.display()
        );
        exit(1);
    }

    // Create a virtual environment
    if let Err(e) = create_virtual_environment() {
        error!("main: {}", e);
        exit(1);
    }

    // Rename the original pyproject.toml to old.pyproject.toml
    if let Err(e) = rename_pyproject(file_path) {
        error!("main: Error renaming pyproject.toml: {}", e);
        exit(1);
    }

    let old_pyproject_path = file_path.with_file_name("old.pyproject.toml");

    // Initialize new project with uv init
    if let Err(e) = create_new_pyproject(project_dir) {
        error!("main: Error initializing new project: {}", e);
        exit(1);
    }

    // Read the contents of the old pyproject.toml
    info!(
        "main: Reading old pyproject.toml from: {}",
        old_pyproject_path.display()
    );
    let contents = match fs::read_to_string(&old_pyproject_path) {
        Ok(c) => c,
        Err(e) => {
            error!(
                "main: Error reading file '{}': {}",
                old_pyproject_path.display(),
                e
            );
            exit(1);
        }
    };

    info!("main: Parsing old pyproject.toml");
    let pyproject: PyProject = match toml::from_str(&contents) {
        Ok(p) => p,
        Err(e) => {
            error!(
                "main: Error parsing TOML in '{}': {}",
                old_pyproject_path.display(),
                e
            );
            exit(1);
        }
    };

    match add_all_dependencies(&pyproject) {
        Ok(_) => info!("main: Successfully processed dependencies"),
        Err(e) => {
            error!("main: Error processing dependencies: {}", e);
            exit(1);
        }
    }
}

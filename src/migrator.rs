use crate::types::PyProject;
use crate::utils::{create_virtual_environment, find_pyproject_toml, format_dependency, should_include_dependency};
use log::{debug, info};
use std::fs;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::fs::OpenOptions;
use std::path::{Path};
use std::process::{Command, exit};

pub fn run_migration(project_dir: &Path) -> Result<(), String> {
    let file_path = match find_pyproject_toml(project_dir) {
        Some(path) => path,
        None => return Err("No pyproject.toml found in the specified directory".to_string()),
    };

    info!("Project directory: {:?}", project_dir);
    info!("pyproject.toml path: {:?}", file_path);

    let contents = fs::read_to_string(&file_path)
        .map_err(|e| format!("Error reading file '{}': {}", file_path.display(), e))?;

    let pyproject: PyProject = toml::from_str(&contents)
        .map_err(|e| format!("Error parsing TOML in '{}': {}", file_path.display(), e))?;

    if !check_for_poetry_section(&pyproject) {
        info!("Poetry section not found in pyproject.toml. Nothing to migrate.");
        exit(0);
    }

    create_virtual_environment()?;
    rename_pyproject(&file_path)?;
    let old_pyproject_path = file_path.with_file_name("old.pyproject.toml");
    create_new_pyproject(project_dir)?;

    add_all_dependencies(&pyproject)?;

    let new_pyproject_path = project_dir.join("pyproject.toml");
    append_tool_sections(&old_pyproject_path, &new_pyproject_path)?;

    Ok(())
}

fn rename_pyproject(pyproject_path: &Path) -> Result<(), String> {
    let old_pyproject_path = pyproject_path.with_file_name("old.pyproject.toml");
    fs::rename(pyproject_path, &old_pyproject_path)
        .map_err(|e| format!("Failed to rename pyproject.toml: {}", e))?;
    info!("Renamed existing pyproject.toml to old.pyproject.toml");
    Ok(())
}

fn create_new_pyproject(project_dir: &Path) -> Result<(), String> {
    info!("Initializing new project with uv init");
    let uv_path = which::which("uv").map_err(|e| format!("Failed to find uv command: {}", e))?;
    let output = Command::new(&uv_path)
        .arg("init")
        .arg("--no-pin-python")
        .current_dir(project_dir)
        .output()
        .map_err(|e| format!("Failed to execute uv init: {}", e))?;

    if output.status.success() {
        info!("Successfully initialized new project with uv init");
        let hello_py_path = project_dir.join("hello.py");
        if hello_py_path.exists() {
            fs::remove_file(&hello_py_path)
                .map_err(|e| format!("Failed to delete hello.py: {}", e))?;
            info!("Deleted hello.py file");
        }
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("uv init failed: {}", stderr))
    }
}

fn add_dependencies(deps: &[String], dev: bool) -> Result<(), String> {
    if deps.is_empty() {
        info!("No {} dependencies to add.", if dev { "dev" } else { "main" });
        return Ok(());
    }

    let uv_path = which::which("uv").map_err(|e| format!("Failed to find uv command: {}", e))?;
    let mut command = Command::new(uv_path);
    command.arg("add");
    if dev {
        command.arg("--dev");
    }
    command.args(deps);

    let output = command
        .output()
        .map_err(|e| format!("Failed to execute uv command: {}", e))?;

    if output.status.success() {
        info!("{} dependencies added successfully!", if dev { "Dev" } else { "Main" });
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to add {} dependencies: {}", if dev { "dev" } else { "main" }, stderr))
    }
}

fn add_all_dependencies(pyproject: &PyProject) -> Result<(), String> {
    let mut main_deps = Vec::new();
    let mut dev_deps = Vec::new();

    if let Some(project) = &pyproject.project {
        // Handle PEP 621 format
        if let Some(deps) = &project.dependencies {
            match deps {
                toml::Value::Table(table) => {
                    main_deps.extend(table.iter().map(|(k, v)| format!("{}=={}", k, v)));
                }
                toml::Value::Array(array) => {
                    main_deps.extend(array.iter().filter_map(|v| v.as_str().map(String::from)));
                }
                _ => return Err("Unsupported dependency format in [project.dependencies]".to_string()),
            }
        }
        if let Some(optional_deps) = &project.optional_dependencies {
            for (_, group_deps) in optional_deps {
                match group_deps {
                    toml::Value::Table(table) => {
                        dev_deps.extend(table.iter().map(|(k, v)| format!("{}=={}", k, v)));
                    }
                    toml::Value::Array(array) => {
                        dev_deps.extend(array.iter().filter_map(|v| v.as_str().map(String::from)));
                    }
                    _ => return Err("Unsupported dependency format in [project.optional-dependencies]".to_string()),
                }
            }
        }
    } else if let Some(tool) = &pyproject.tool {
        if let Some(poetry) = &tool.poetry {
            // Handle Poetry format
            if let Some(deps) = &poetry.dependencies {
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
                for (_, group) in groups {
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
            return Err("No [tool.poetry] section found in pyproject.toml".to_string());
        }
    } else {
        return Err("Neither [project] (PEP 621) nor [tool.poetry] section found in pyproject.toml".to_string());
    }

    debug!("Main dependencies: {:?}", main_deps);
    debug!("Dev dependencies: {:?}", dev_deps);

    main_deps.sort();
    main_deps.dedup();
    dev_deps.sort();
    dev_deps.dedup();

    add_dependencies(&main_deps, false)?;
    add_dependencies(&dev_deps, true)?;

    Ok(())
}

fn check_for_poetry_section(pyproject: &PyProject) -> bool {
    if let Some(tool) = &pyproject.tool {
        if tool.poetry.is_some() {
            info!("[tool.poetry] section found in pyproject.toml");
            return true;
        }
    }
    info!("[tool.poetry] section not found in pyproject.toml");
    false
}

fn append_tool_sections(old_pyproject_path: &Path, new_pyproject_path: &Path) -> Result<(), String> {
    let old_file = BufReader::new(fs::File::open(old_pyproject_path)
        .map_err(|e| format!("Failed to open old pyproject.toml: {}", e))?);

    let mut new_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(new_pyproject_path)
        .map_err(|e| format!("Failed to open new pyproject.toml for reading and writing: {}", e))?;

    let mut in_tool_section = false;
    let mut is_poetry_section = false;
    let mut current_section = String::new();
    let mut tool_sections = String::new();
    let mut existing_tool_sections = Vec::new();

    // First, read the new file to check for existing [tool] sections
    let mut new_file_content = String::new();
    new_file.read_to_string(&mut new_file_content)
        .map_err(|e| format!("Failed to read new pyproject.toml: {}", e))?;

    for line in new_file_content.lines() {
        if line.starts_with("[tool.") && !line.starts_with("[tool.poetry") {
            existing_tool_sections.push(line.to_string());
        }
    }

    // Now process the old file
    for line in old_file.lines() {
        let line = line.map_err(|e| format!("Error reading line: {}", e))?;

        if line.starts_with("[tool.") {
            // If we were in a non-poetry tool section, add it
            if in_tool_section && !is_poetry_section
                && !existing_tool_sections.contains(&current_section.lines().next().unwrap_or("").to_string()) {
                tool_sections.push_str(&current_section);
            }
            in_tool_section = true;
            is_poetry_section = line.starts_with("[tool.poetry") || is_poetry_section;
            current_section = String::new();
            if !is_poetry_section {
                current_section.push_str(&line);
                current_section.push_str("\n");
            }
        } else if line.starts_with('[') {
            // If we were in a non-poetry tool section, add it
            if in_tool_section && !is_poetry_section
                && !existing_tool_sections.contains(&current_section.lines().next().unwrap_or("").to_string()) {
                tool_sections.push_str(&current_section);
            }
            in_tool_section = false;
            is_poetry_section = false;
            current_section.clear();
        } else if in_tool_section && !is_poetry_section {
            current_section.push_str(&line);
            current_section.push_str("\n");
        }
    }

    // Check the last section
    if in_tool_section && !is_poetry_section
        && !existing_tool_sections.contains(&current_section.lines().next().unwrap_or("").to_string()) {
        tool_sections.push_str(&current_section);
    }

    if !tool_sections.is_empty() {
        new_file.seek(SeekFrom::End(0)).map_err(|e| format!("Failed to seek to end of file: {}", e))?;
        writeln!(new_file).map_err(|e| format!("Failed to write newline: {}", e))?;
        write!(new_file, "{}", tool_sections.trim_start())
            .map_err(|e| format!("Failed to write tool sections: {}", e))?;
        println!("Appended [tool] sections to new pyproject.toml");
    } else {
        println!("No new [tool] sections found to append");
    }

    Ok(())
}
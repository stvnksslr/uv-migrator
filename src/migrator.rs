use crate::types::PyProject;
use crate::utils::{create_virtual_environment, find_pyproject_toml, format_dependency, should_include_dependency};
use log::{debug, info, warn};
use std::fs;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run_migration(project_dir: &Path) -> Result<(), String> {
    info!("Project directory: {:?}", project_dir);

    create_virtual_environment()?;

    let (main_deps, dev_deps, had_pyproject) = detect_and_migrate(project_dir)?;

    // Create new pyproject.toml
    create_new_pyproject(project_dir)?;

    // Add dependencies
    add_all_dependencies(&main_deps, &dev_deps)?;

    // Append tool sections if migrated from pyproject.toml
    if had_pyproject {
        let old_pyproject_path = project_dir.join("old.pyproject.toml");
        let new_pyproject_path = project_dir.join("pyproject.toml");
        if old_pyproject_path.exists() {
            append_tool_sections(&old_pyproject_path, &new_pyproject_path)?;
        } else {
            info!("Expected old pyproject.toml not found. Skipping tool section appending.");
        }
    } else {
        info!("No original pyproject.toml found. Skipping tool section appending.");
    }

    Ok(())
}

fn detect_and_migrate(project_dir: &Path) -> Result<(Vec<String>, Vec<String>, bool), String> {
    let pyproject_path = find_pyproject_toml(project_dir);

    if let Some(pyproject_path) = pyproject_path {
        if has_poetry_section(&pyproject_path)? {
            info!("Detected Poetry project. Migrating from pyproject.toml");
            let (main_deps, dev_deps) = migrate_from_pyproject(&pyproject_path)?;
            Ok((main_deps, dev_deps, true))
        } else {
            warn!("pyproject.toml found but no Poetry section detected. Checking for requirements.txt");
            let (main_deps, dev_deps) = migrate_from_requirements(project_dir)?;
            Ok((main_deps, dev_deps, false))
        }
    } else {
        info!("No pyproject.toml found. Checking for requirements files.");
        let (main_deps, dev_deps) = migrate_from_requirements(project_dir)?;
        Ok((main_deps, dev_deps, false))
    }
}

fn has_poetry_section(pyproject_path: &Path) -> Result<bool, String> {
    let contents = fs::read_to_string(pyproject_path)
        .map_err(|e| format!("Error reading file '{}': {}", pyproject_path.display(), e))?;

    let pyproject: PyProject = toml::from_str(&contents)
        .map_err(|e| format!("Error parsing TOML in '{}': {}", pyproject_path.display(), e))?;

    Ok(pyproject.tool.and_then(|t| t.poetry).is_some())
}

fn migrate_from_pyproject(pyproject_path: &Path) -> Result<(Vec<String>, Vec<String>), String> {
    let contents = fs::read_to_string(pyproject_path)
        .map_err(|e| format!("Error reading file '{}': {}", pyproject_path.display(), e))?;

    let pyproject: PyProject = toml::from_str(&contents)
        .map_err(|e| format!("Error parsing TOML in '{}': {}", pyproject_path.display(), e))?;

    let (main_deps, dev_deps) = extract_dependencies_from_pyproject(&pyproject)?;

    rename_pyproject(pyproject_path)?;

    Ok((main_deps, dev_deps))
}

fn migrate_from_requirements(project_dir: &Path) -> Result<(Vec<String>, Vec<String>), String> {
    let requirements_files = find_requirements_files(project_dir);

    if requirements_files.is_empty() {
        return Err("No requirements files found. Please ensure you have either a requirements.txt file or a pyproject.toml with a [tool.poetry] section.".to_string());
    }

    let mut main_deps = Vec::new();
    let mut dev_deps = Vec::new();

    for file_path in &requirements_files {
        let file_name = file_path.file_name().unwrap().to_str().unwrap();
        if file_name == "requirements.txt" {
            main_deps.push(file_path.to_str().unwrap().to_string());
        } else {
            dev_deps.push(file_path.to_str().unwrap().to_string());
        }
        info!("Found requirements file: {}", file_path.display());
    }

    Ok((main_deps, dev_deps))
}

fn find_requirements_files(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() && path.file_name().unwrap().to_str().unwrap().starts_with("requirements") {
                Some(path)
            } else {
                None
            }
        })
        .collect()
}

fn extract_dependencies_from_pyproject(pyproject: &PyProject) -> Result<(Vec<String>, Vec<String>), String> {
    let mut main_deps = Vec::new();
    let mut dev_deps = Vec::new();

    if let Some(tool) = &pyproject.tool {
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
        return Err("No [tool] section found in pyproject.toml".to_string());
    }

    Ok((main_deps, dev_deps))
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

    for dep in deps {
        command.arg("--requirements").arg(dep);
    }

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


fn add_all_dependencies(main_deps: &[String], dev_deps: &[String]) -> Result<(), String> {
    debug!("Main requirements files: {:?}", main_deps);
    debug!("Dev requirements files: {:?}", dev_deps);

    add_dependencies(main_deps, false)?;
    add_dependencies(dev_deps, true)?;

    Ok(())
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
            is_poetry_section = line.starts_with("[tool.poetry");
            current_section = String::new();
            if !is_poetry_section {
                current_section.push_str(&line);
                current_section.push('\n');
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
            current_section.push('\n');
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
        info!("Appended [tool] sections to new pyproject.toml");
    } else {
        info!("No new [tool] sections found to append");
    }

    Ok(())
}
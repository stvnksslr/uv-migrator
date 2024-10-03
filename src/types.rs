use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub struct PyProject {
    pub project: Option<Project>,
    pub tool: Option<Tool>,
}

#[derive(Deserialize, Debug)]
pub struct Project {
    pub dependencies: Option<toml::Value>,
    pub optional_dependencies: Option<HashMap<String, toml::Value>>,
}

#[derive(Deserialize, Debug)]
pub struct Tool {
    pub poetry: Option<Poetry>,
}

#[derive(Deserialize, Debug)]
pub struct Poetry {
    pub dependencies: Option<HashMap<String, toml::Value>>,
    pub group: Option<HashMap<String, Group>>,
}

#[derive(Deserialize, Debug)]
pub struct Group {
    pub dependencies: HashMap<String, toml::Value>,
}
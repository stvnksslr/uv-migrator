#[derive(Debug, PartialEq, Clone)]
pub enum DependencyType {
    Main,
    Dev,
}

#[derive(Debug)]
pub struct Dependency {
    pub name: String,
    pub version: Option<String>,
    pub dep_type: DependencyType,
    pub environment_markers: Option<String>,
}

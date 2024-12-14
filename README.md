# UV Migrator

## Disclaimer

This project is not associated with astral or the uv project in anyway

## What is it?

UV Migrator is simple cli tool designed to seamlessly transition Python projects from various dependency management systems to the UV package manager. It handles the complexities of migration while preserving your project's dependencies and any existing configs.

## Installation

easy install script, source located at [install.sh](https://github.com/stvnksslr/uv-migrator/blob/main/Cargo.toml)

```sh
curl https://uv-migrator.stvnksslr.com/install.sh | bash
```

Install via Cargo

```sh
cargo install uv-migrator
```

## Currently Supported

✅ Poetry projects  
✅ Pip based projects  
✅ Multiple requirements files  
✅ Auto Detect Development dependencies  
✅ Dependency groups  
✅ Custom package indexes  

## Coming Soon

🔄 PDM support  
🔄 Hatch support  
🔄 Pipenv support  

## Usage

Run the UV Migrator with the path to your existing project directory:

```sh
uv-migrator path/to/your/project
```

or

```sh
cd /to/project
uv-migrator .
```

### Additional Options

```sh
Usage: uv-migrator [OPTIONS] [PATH]

Arguments:
  [PATH]  The path to the project directory to migrate

Options:
      --self-update                  Update uv-migrator to the latest version
      --import-global-pip-conf       Import extra index URLs from ~/.pip/pip.conf
      --import-index <import-index>  Additional index URL to import
      --merge-groups                 Merge all dependency groups into the dev group
  -h, --help                         Print help (see more with '--help')
  -V, --version                      Print version

EXAMPLES:
# Migrate a project in the current directory
uv-migrator .

# Migrate a project with a private package index
uv-migrator . --import-index https://private.pypi.org/simple/

# Update uv-migrator to the latest version
uv-migrator --self-update

# Migrate using global pip configuration
uv-migrator . --import-global-pip-conf

# Merge all dependency groups into dev dependencies
uv-migrator . --merge-groups
```

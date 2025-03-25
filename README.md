# UV Migrator

## Disclaimer

This project is not associated with astral or the uv project in anyway

## What is it?

UV Migrator is simple cli tool designed to seamlessly transition Python projects from various dependency management systems to the UV package manager.
It handles the complexities of migration while preserving your project's dependencies and any existing configs. This project currently supports migrating
applications that consume packages, stay tuned for support for migrating packages themselves.

## Installation

easy install script, source located at [install.sh](https://github.com/stvnksslr/uv-migrator/blob/main/uv-migrator/install.sh)

```sh
curl https://files.stvnksslr.com/uv-migrator/install.sh | bash
```

Install via Cargo

```sh
cargo install uv-migrator
```

## Currently Supported

✅ Poetry projects  
✅ Pip projects  
✅ Multiple requirements files  
✅ Auto detect development dependencies and dependency groups  
✅ Custom package indexes  
✅ Pipenv support

Package Formats  
✅ setup.py packages  
✅ poetry packages

## Coming Soon

Project formats  
🔄 anaconda projects

## Usage

```sh
❯ uv-migrator -h
A tool for migrating Python projects to use the uv package manager

Usage: uv-migrator [OPTIONS] [PATH]

Arguments:
  [PATH]  The path to the project directory to migrate [default: .]

Options:
      --merge-groups                 Merge all dependency groups into the dev group
      --import-global-pip-conf       Import extra index URLs from ~/.pip/pip.conf
      --import-index <import-index>  Additional index URL to import
      --disable-restore              Disable automatic file restore on error
      --self-update                  Update uv-migrator to the latest version
      --check-update                 Check for updates without installing them
  -h, --help                         Print help (see more with '--help')
  -V, --version                      Print version

EXAMPLES:
# Migrate a project in the current directory
uv-migrator .

# Merge all dependency groups into dev dependencies
uv-migrator . --merge-groups

# Migrate a project with a private package index
uv-migrator . --import-index https://private.pypi.org/simple/

# Migrate using global pip configuration
uv-migrator . --import-global-pip-conf

# Migrate without automatic restore on error
uv-migrator . --disable-restore

# Check for updates without installing them
uv-migrator --check-update

# Update to the latest version
uv-migrator --self-update

For more information and documentation, visit:
https://github.com/stvnksslr/uv-migrator
```

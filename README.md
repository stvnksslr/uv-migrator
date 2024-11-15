# UV Migrator

UV Migrator is a Rust-based tool designed to facilitate the migration of Python projects from various dependency
management systems (like pip or poetry) to the UV package manager. This tool automates the process of creating a new
UV-based project structure while preserving existing dependencies.

## Disclaimer

This project is not associated with astral or the uv project in anyway, please use at your own risk and keep backups of
your dependency declarations for reference

## Features

- Supports migration from Poetry and PEP 621 project structures
- Creates a new virtual environment using UV
- Automatically transfers dependencies from the existing `pyproject.toml` or `requirements.txt` to the new UV-based
  project
- Attempts to migrate all [tool.*] configs to the new `pyproject.toml` file
- Handles both main and development dependencies
- Provides detailed logging for transparency and debugging
- Supports importing extra index URLs from global pip configuration
- Allows specifying additional index URLs during migration
- By default, doesn't pin the python version via a .python-versions file in case the user uses asdf/mise and
  .tool-versions files

## Prerequisites

Before you begin, ensure you have the following installed:

- Rust (latest stable version)
- UV package manager

## Installation

```
cargo install uv-migrator
```

The compiled binary will be available in the `target/release` directory.

## Supported Package Managers

* poetry
* pip (requirements.txt)

## In Progress

* pdm
* hatch
* pipenv
* Open Issues for more!

## Usage

Run the UV Migrator with the path to your existing project directory:

```
uv-migrator path/to/your/project
```

or

```
1. cd /to/project
2. uv-migrator .
```

### Additional Options

- `--import-global-pip-conf`: Import extra index URLs from `~/.pip/pip.conf`
  ```bash
  uv-migrator path/to/your/project --import-global-pip-conf
  ```

- `--import-index`: Specify additional index URLs to import (can be used multiple times)
  ```bash
  uv-migrator path/to/your/project --import-index https://custom.pypi.org/simple/
  ```

- `--help`: Display comprehensive help information
  ```bash
  uv-migrator --help
  ```

- `--self-update`: Update UV Migrator to the latest version
  ```bash
  uv-migrator --self-update
  ```
# UV Migrator

UV Migrator is a Rust-based tool designed to facilitate the migration of Python projects from various dependency
management systems (like Poetry or PEP 621) to the UV package manager. This tool automates the process of creating a new
UV-based project structure while preserving existing dependencies.

## Features

- Supports migration from Poetry and PEP 621 project structures
- Creates a new virtual environment using UV
- Automatically transfers dependencies from the existing `pyproject.toml` to the new UV-based project
- Attempts to migrate all [tool.*] configs to the new `pyproject.toml` file
- Handles both main and development dependencies
- Provides detailed logging for transparency and debugging

## Prerequisites

Before you begin, ensure you have the following installed:

- Rust (latest stable version)
- UV package manager

## Installation

`cargo install uv-migrator`

The compiled binary will be available in the `target/release` directory.

## Supported Package Managers

* poetry
* pip

## In Progress

* PDM
* Hatch
* pipenv
* Open Issues for more!

## Usage

Run the UV Migrator with the path to your existing `pyproject.toml` file:

```
uv-migrator path/to/your/pyproject.toml/location
```

or

```
1. cd /to/project
2. uv-migrator .
```

## How It Works

1. Renames your existing `pyproject.toml` to `old.pyproject.toml`.
2. A new UV-based project is initialized in the same directory.
3. The tool parses the `old.pyproject.toml` file and extracts all dependencies.
4. These dependencies are then installed in the new UV-based project, maintaining the distinction between main and
   development dependencies.
5. all additional configs from a pyproject.toml will need to be added by hand
# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

UV Migrator is a Rust CLI tool that migrates Python projects from various dependency management systems (Poetry, Pipenv, requirements.txt, setup.py, Conda) to the UV package manager. It handles dependency extraction, project initialization, and configuration migration while preserving existing project settings.

**Important**: This project is not associated with Astral or the uv project.

## Development Commands

### Building and Testing

```bash
# Build the project (requires Rust/Cargo via mise)
cargo build

# Run tests
cargo test

# Run a specific test
cargo test <test_name>

# Build release version
cargo build --release

# Run the CLI locally
cargo run -- [OPTIONS] [PATH]
```

### Installation Options

Users can install via:
- Install script: `curl https://files.stvnksslr.com/uv-migrator/install.sh | bash`
- Cargo: `cargo install uv-migrator`

## Architecture

### Core Migration Flow

The migration process follows a trait-based architecture with two main abstractions:

1. **MigrationSource** (src/migrators/mod.rs:20-23): Extracts dependencies from source projects
2. **MigrationTool** (src/migrators/mod.rs:26-37): Prepares projects and adds dependencies

Main entry point: `run_migration()` in src/migrators/mod.rs:307

### Migration Workflow

1. **Detection** (src/migrators/detect.rs): Auto-detects project type via file presence:
   - Conda: environment.yml or environment.yaml
   - Poetry: pyproject.toml with [tool.poetry] or [project] sections
   - Pipenv: Pipfile
   - setup.py: setup.py file
   - Requirements: requirements*.txt files

2. **Extraction**: Project-specific migrators extract dependencies into common `Dependency` model

3. **Initialization**: `UvTool` prepares new UV project:
   - Backs up existing pyproject.toml to old.pyproject.toml
   - Runs `uv init` with appropriate flags (--package for packages, --bare for newer UV versions)
   - Handles Python version constraints from original project

4. **Migration**: Adds dependencies via `uv add` grouped by type (Main/Dev/Group)

5. **Cleanup**: Project-specific post-processing and file cleanup

### Key Components

**Models** (src/models/):
- `Dependency`: Core dependency representation with name, version, type (Main/Dev/Group), extras, environment markers
- `ProjectType`: Enum for Poetry/Pipenv/Requirements/SetupPy/Conda
- `PoetryProjectType`: Distinguishes Application vs Package projects

**Migrators** (src/migrators/):
- Each source has its own migrator implementing `MigrationSource`
- `common.rs`: Shared migration logic for handling custom package indexes, author info, build systems
- `format_dependency()`: Converts internal Dependency model to UV CLI format

**Utils** (src/utils/):
- `file_ops.rs`: FileTracker system for automatic rollback on migration errors
- `uv.rs`: UvCommandBuilder for constructing UV CLI commands
- `pyproject.rs`: TOML parsing and manipulation helpers
- `pip.rs`: Handles ~/.pip/pip.conf parsing for custom indexes

**Error Handling** (src/error.rs):
- Custom error types for file operations, UV commands, dependencies, TOML parsing
- FileTrackerGuard implements automatic rollback via Drop trait when restore_enabled=true

### Version Compatibility

The tool checks UV version (src/migrators/mod.rs:127-156) to determine feature availability:
- UV >= 0.5.11: Uses `--bare` flag to avoid creating hello.py
- Older versions: Manually cleans up hello.py after init

### Important Implementation Details

**Poetry Package Detection** (src/migrators/poetry.rs):
- Checks for actual package structure (src/<package> or lib directory)
- Projects with `packages` config but no real package structure are treated as applications
- Original project type preserved for migrating package configs to Hatchling format

**Dependency Formatting** (src/migrators/mod.rs:271-305):
- Converts Poetry `^` constraints to `>=`
- Converts `~` to `~=`
- Handles extras, environment markers, and complex version specs

**File Tracking** (src/utils/file_ops.rs):
- All file operations tracked for automatic rollback
- Stores original content for modified files
- Rollback triggered on error or via `force_rollback()`

**Custom Package Indexes**:
- Supports named indexes via `--import-index name@url` format
- Can import from global ~/.pip/pip.conf via `--import-global-pip-conf`
- Migrated to `[tool.uv.index]` section in pyproject.toml

## Testing Strategy

Tests are integration-focused, using tempfile for isolated test environments:
- Each migrator has dedicated test file (poetry_test.rs, pipenv_test.rs, etc.)
- Tests verify complete migration flow: source → UV project
- Specific tests for edge cases (git dependencies, package configs, version constraints)
- `dependency_format_test.rs`: Tests dependency string formatting
- `file_tracker_test.rs`: Tests rollback functionality

## CLI Options

Key flags:
- `--merge-groups`: Consolidates all dependency groups into dev dependencies
- `--import-global-pip-conf`: Import extra-index-url from ~/.pip/pip.conf
- `--import-index [name@]url`: Add custom package indexes
- `--disable-restore`: Skip automatic rollback on errors
- `--self-update`: Update uv-migrator to latest version
- `--check-update`: Check for updates without installing

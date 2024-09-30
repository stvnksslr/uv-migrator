# UV Migrator

UV Migrator is a Rust-based tool designed to facilitate the migration of Python projects from various dependency management systems (like Poetry or PEP 621) to the UV package manager. This tool automates the process of creating a new UV-based project structure while preserving existing dependencies.

## Features

- Supports migration from Poetry and PEP 621 project structures
- Creates a new virtual environment using UV
- Automatically transfers dependencies from the existing `pyproject.toml` to the new UV-based project
- Handles both main and development dependencies
- Provides detailed logging for transparency and debugging

## Prerequisites

Before you begin, ensure you have the following installed:
- Rust (latest stable version)
- UV package manager

## Installation

1. Clone the repository:
   ```
   git clone https://github.com/yourusername/uv-migrator.git
   cd uv-migrator
   ```

2. Build the project:
   ```
   cargo build --release
   ```

The compiled binary will be available in the `target/release` directory.

## Usage

Run the UV Migrator with the path to your existing `pyproject.toml` file:

```
./target/release/uv-migrator path/to/your/pyproject.toml
```

For more detailed logging, you can set the `RUST_LOG` environment variable:

```
RUST_LOG=debug ./target/release/uv-migrator path/to/your/pyproject.toml
```

## How It Works

1. Renames your existing `pyproject.toml` to `old.pyproject.toml`.
2. A new UV-based project is initialized in the same directory.
3. The tool parses the `old.pyproject.toml` file and extracts all dependencies.
4. These dependencies are then installed in the new UV-based project, maintaining the distinction between main and development dependencies.

## Configuration

The UV Migrator doesn't require any additional configuration. It derives all necessary information from your existing `pyproject.toml` file.

## Troubleshooting

If you encounter any issues:

1. Ensure you have the latest version of UV installed.
2. Check that your `pyproject.toml` file is valid and follows either the Poetry or PEP 621 format.
3. Run the tool with debug logging enabled (`RUST_LOG=debug`) for more detailed output.
4. If the issue persists, please open an issue on the GitHub repository with the full debug output and a description of the problem.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

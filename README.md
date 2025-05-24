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
✅ Custom package indexes with named configurations  
✅ Pipenv support

Package Formats  
✅ setup.py packages  
✅ poetry packages  
✅ anaconda

## Usage

```sh
❯ uv-migrator -h
A tool for migrating Python projects to use the uv package manager

Usage: uv-migrator [OPTIONS] [PATH]

Arguments:
  [PATH]  The path to the project directory to migrate [default: .]

Options:
      --merge-groups                       Merge all dependency groups into the dev group
      --import-global-pip-conf             Import extra index URLs from ~/.pip/pip.conf
      --import-index <import-index>        Additional index URL to import (format: [name@]url)
      --disable-restore                    Disable automatic file restore on error
      --self-update                        Update uv-migrator to the latest version
      --check-update                       Check for updates without installing them
  -h, --help                               Print help (see more with '--help')
  -V, --version                            Print version

EXAMPLES:
# Migrate a project in the current directory
uv-migrator .

# Merge all dependency groups into dev dependencies
uv-migrator . --merge-groups

# Migrate a project with a private package index
uv-migrator . --import-index https://private.pypi.org/simple/

# Migrate with named custom indexes
uv-migrator . --import-index mycompany@https://pypi.mycompany.com/simple/ \
             --import-index torch@https://download.pytorch.org/whl/cu118

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

## Custom Index Configuration

UV Migrator supports custom package indexes with named configurations. You can specify custom names for your indexes using the `[name@]url` format:

### Named Indexes

```sh
# Add a named index
uv-migrator . --import-index mycompany@https://pypi.mycompany.com/simple/

# Add multiple named indexes
uv-migrator . --import-index torch@https://download.pytorch.org/whl/cu118 \
             --import-index internal@https://internal.company.com/pypi/
```

This will generate:

```toml
[tool.uv]
index = [
    { name = "torch", url = "https://download.pytorch.org/whl/cu118" },
    { name = "internal", url = "https://internal.company.com/pypi/" }
]
```

### Index Name Format

- Names can contain letters, numbers, hyphens, and underscores
- The `@` symbol separates the name from the URL
- If no name is provided, or the format is invalid, the URL is treated as unnamed

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
  [PATH]
          Specifies the directory containing the Python project to migrate. This should be the root directory of your project where pyproject.toml or requirements.txt is located.

Options:
  --self-update
      Checks for and downloads the latest version of uv-migrator from GitHub releases. The tool will automatically update itself if a newer version is available.

  --import-global-pip-conf
      Reads and imports any extra package index URLs defined in your global pip configuration file (~/.pip/pip.conf). This is useful when your project requires packages from private or alternative Python package indexes.

  --import-index <import-index>
      Specifies additional Python package index URLs to use. You can provide this option multiple times to add several index URLs. These URLs will be added to your project's pyproject.toml in the [tool.uv] section.

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

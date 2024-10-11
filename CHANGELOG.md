# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
## [2024.3.1](https://github.com/stvnksslr/uv-migrator/compare/v2024.3.0...v2024.3.1) - 2024-10-11

### Fixed
- *(multiple requirements files)* req.txt files were not properly being sorted into main and dev dependencies and the search code often missed extras (by @stvnksslr)

### Contributors

* @stvnksslr
## [2024.3.0](https://github.com/stvnksslr/uv-migrator/compare/v2024.2.11...v2024.3.0) - 2024-10-10

### Added
- *(pip)* requirements.txt is handled better + cases with no existing pyproject.toml is handled (by @stvnksslr)

### Other
- *(readme)* adding pipenv to scope of migration (by @stvnksslr)
- *(migrator)* extracting logic and breaking each implementation into more focused implementations (by @stvnksslr)

### Contributors

* @stvnksslr
## [2024.2.11](https://github.com/stvnksslr/uv-migrator/compare/v2024.2.10...v2024.2.11) - 2024-10-10

### Fixed
- *(releases)* env incorrectly used (by @stvnksslr)

### Other
- chore(removing unused envs): (by @stvnksslr)

### Contributors

* @stvnksslr
## [2024.2.10](https://github.com/stvnksslr/uv-migrator/compare/v2024.2.9...v2024.2.10) - 2024-10-10

### Fixed
- *(releases)* env incorrectly used (by @stvnksslr)

### Contributors

* @stvnksslr
## [2024.2.9](https://github.com/stvnksslr/uv-migrator/compare/v2024.2.8...v2024.2.9) - 2024-10-10

### Added
- *(workflow fixes + removing goreleaser)* goreleaser is really cool but its a bit of an odd hoop jump for releaser (by @stvnksslr)

### Contributors

* @stvnksslr
## [2024.2.8](https://github.com/stvnksslr/uv-migrator/compare/v2024.2.7...v2024.2.8) - 2024-10-10

### Fixed
- fix(errors in release flow): (by @stvnksslr)

### Contributors

* @stvnksslr
## [2024.2.7](https://github.com/stvnksslr/uv-migrator/compare/v2024.2.6...v2024.2.7) - 2024-10-10

### Other
- *(releases)* fully removing deprecated workflows (by @stvnksslr)

### Contributors

* @stvnksslr
## [2024.2.6](https://github.com/stvnksslr/uv-migrator/compare/v2024.2.5...v2024.2.6) - 2024-10-10

### Other
- *(actions-rs)* actions-rs is a deprecated org and all their actions are not currently updated, migrating over to maintained versions (by @stvnksslr)

### Contributors

* @stvnksslr
## [2024.2.5](https://github.com/stvnksslr/uv-migrator/compare/v2024.2.4...v2024.2.5) - 2024-10-10

### Other
- wip (by @stvnksslr)

### Contributors

* @stvnksslr
## [2024.2.4](https://github.com/stvnksslr/uv-migrator/compare/v2024.2.3...v2024.2.4) - 2024-10-10

### Added
- *(release flow)* unified cargo + github release with multi arch binaries and change log (by @stvnksslr)

### Contributors

* @stvnksslr
## [2024.2.3](https://github.com/stvnksslr/uv-migrator/compare/v2024.2.2...v2024.2.3) - 2024-10-10

### Added
- *(release flow)* unified cargo + github release with multi arch binaries and change log (by @stvnksslr)

### Contributors

* @stvnksslr
## [2024.2.2](https://github.com/stvnksslr/uv-migrator/compare/v2024.2.1...v2024.2.2) - 2024-10-09

### Added
- *(release flow)* unified cargo + github release with multi arch binaries and change log (by @stvnksslr)

### Contributors

* @stvnksslr

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
## [2025.0.0](https://github.com/stvnksslr/uv-migrator/compare/v2024.6.0...v2025.0.0) - 2024-11-15

### Added
- *(dependency groups)* dependency groups are now properly supported by uv, if you are using some form of grouping requirements-test.txt or poetry  [tool.poetry.group.test.dependencies] these will now carry over, if you are not using them everything will fall under --dev as normal (by @stvnksslr)

### Fixed
- *(poetry)* was not clearly translating poetry dep groups to uv ones in many circumstances (by @stvnksslr)

### Other
- chore(gitignore tweaks): (by @stvnksslr)
- chore(gitignore tweaks): (by @stvnksslr)

### Contributors

* @stvnksslr
## [2024.6.0](https://github.com/stvnksslr/uv-migrator/compare/v2024.5.4...v2024.6.0) - 2024-11-11

### Added
- *(poetry-migrator)* added ability to pull in the description from poetry packages (by @stvnksslr)

### Fixed
- *(linter)* clippy suggestions (by @stvnksslr)

### Contributors

* @stvnksslr
## [2024.5.4](https://github.com/stvnksslr/uv-migrator/compare/v2024.5.3...v2024.5.4) - 2024-11-11

### Fixed
- *(openssl)* one last round of fixes this time for windows (by @stvnksslr)

### Contributors

* @stvnksslr
## [2024.5.2](https://github.com/stvnksslr/uv-migrator/compare/v2024.5.1...v2024.5.2) - 2024-11-11

### Fixed
- *(self-update)* incorrect features to upgrade from releases .tar/.zip (by @stvnksslr)

### Contributors

* @stvnksslr
## [2024.5.1](https://github.com/stvnksslr/uv-migrator/compare/v2024.5.0...v2024.5.1) - 2024-11-11

### Other
- Fix/openssl issues ([#20](https://github.com/stvnksslr/uv-migrator/pull/20)) (by @stvnksslr)
- chore(readme): (by @stvnksslr)

### Contributors

* @stvnksslr
## [2024.4.0](https://github.com/stvnksslr/uv-migrator/compare/v2024.3.4...v2024.4.0) - 2024-11-10

### Added
- *(tests)* requirements.txt and poetry parser and migration tests, these should have been in from the getgo but will also work as a better frame of reference for other tools (by @stvnksslr)

### Other
- *(readme)* minor readme tune up (by @stvnksslr)

### Contributors

* @stvnksslr
## [2024.3.4](https://github.com/stvnksslr/uv-migrator/compare/v2024.3.3...v2024.3.4) - 2024-11-09

### Other
- *(poetry)* dependency parsing issues, issues paring ^, >=, <= in dependencies in certain situations (by @stvnksslr)

### Contributors

* @stvnksslr
## [2024.3.3](https://github.com/stvnksslr/uv-migrator/compare/v2024.3.2...v2024.3.3) - 2024-10-27

### Other
- *(* character)* wildcard or all on dependencies now handled properly when importing (by @stvnksslr)
- *(readme)* fixing some readme language (by @stvnksslr)
- chore(deps bump + readme): (by @stvnksslr)

### Contributors

* @stvnksslr
## [2024.3.2](https://github.com/stvnksslr/uv-migrator/compare/v2024.3.1...v2024.3.2) - 2024-10-21

### Added
- *(private package indexes)* because of the way the migrator tool is setup it requires adding the packages and creating a lockfile which can cause issues if a given package cannot be seen due to being in a private index this can be fixed by fetching from a global pip.conf or providing a index url (by @stvnksslr)

### Other
- *(changing defaults)* By default doesnt pin the python version via a .python-versions file incase the user uses asdf/mise and .tool-versions files (by @stvnksslr)

### Contributors

* @stvnksslr
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

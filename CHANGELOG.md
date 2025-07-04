# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
## [2025.8.2](https://github.com/stvnksslr/uv-migrator/compare/v2025.8.1...v2025.8.2) - 2025-06-20

### Fixed
- *(poetry)* python version constraints parsing bug
## [2025.8.1](https://github.com/stvnksslr/uv-migrator/compare/v2025.8.0...v2025.8.1) - 2025-06-06

### Fixed
- fix(misc conda issues):

### Other
- chore(fixing install script source):
## [2025.8.0](https://github.com/stvnksslr/uv-migrator/compare/v2025.7.1...v2025.8.0) - 2025-05-24

### Added
- feat([#72](https://github.com/stvnksslr/uv-migrator/pull/72)): adding named indexes functionality (by @stvnksslr)
- *(conda)* adding conda support (by @stvnksslr)

### Fixed
- fix([#76](https://github.com/stvnksslr/uv-migrator/pull/76)): didnt have correct logic for hatch conversion (by @stvnksslr)
- add indexes before installing dependencies ([#73](https://github.com/stvnksslr/uv-migrator/pull/73)) (by @benedikt-bartscher)

### Other
- *(readme)* changing install url to support additional tools distributed from the same bucket/domain (by @stvnksslr)
- *(readme)* changing install url to support additional tools distributed from the same bucket/domain (by @stvnksslr)
- chore(cleaning up comments): (by @stvnksslr)

### Contributors

* @stvnksslr
* @benedikt-bartscher
## [2025.7.1](https://github.com/stvnksslr/uv-migrator/compare/v2025.7.0...v2025.7.1) - 2025-02-27

### Other
- *(#70)* not parsing git urls or packages correctly (by @stvnksslr)
- *(pipenv)* pipenv was pulling dependencies from the lock file in excess of the top level dependencies in the Pipfile this has been correctd (by @stvnksslr)
- *(goreleaser)* removing deprecated config (by @stvnksslr)

### Contributors

* @stvnksslr
## [2025.6.3](https://github.com/stvnksslr/uv-migrator/compare/v2025.6.2...v2025.6.3) - 2025-02-23

### Fixed
- *(#63)* missed a code path for checking the script name (by @stvnksslr)

### Contributors

* @stvnksslr
## [2025.6.2](https://github.com/stvnksslr/uv-migrator/compare/v2025.6.1...v2025.6.2) - 2025-02-23

### Fixed
- *(#63)* add warning to explain why this was done (by @stvnksslr)
- *(#63)* fix edge case with uv creating a base python script that can clash with system python (by @stvnksslr)

### Other
- *(clippy --fix issue)* when updating to the rust 2024 edition clippy overzelously removed a logging config (by @stvnksslr)
- *(clippy --fix issue)* when updating to the rust 2024 edition clippy overzelously removed a logging config (by @stvnksslr)

### Contributors

* @stvnksslr
## [2025.6.1](https://github.com/stvnksslr/uv-migrator/compare/v2025.6.0...v2025.6.1) - 2025-02-21

### Fixed
- *(readme)* minor formatting change (by @stvnksslr)

### Other
- chore(rust 1.85 + rust 2024 edition): (by @stvnksslr)

### Contributors

* @stvnksslr
## [2025.6.0](https://github.com/stvnksslr/uv-migrator/compare/v2025.5.1...v2025.6.0) - 2025-02-03

### Added
- *(#57)* reduces the number of files created in all variations of flags and workflows (by @stvnksslr)

### Contributors

* @stvnksslr
## [2025.5.1](https://github.com/stvnksslr/uv-migrator/compare/v2025.5.0...v2025.5.1) - 2025-02-03

### Added
- *(#58 - disable-restore)* disable-restore/rollback flag added for easier time doing either incremental migrations or debugging issues with the process (by @stvnksslr)

### Contributors

* @stvnksslr
## [2025.5.0](https://github.com/stvnksslr/uv-migrator/compare/v2025.4.0...v2025.5.0) - 2025-01-26

### Added
- *(#55)* updates the migrator to handle the poetry v2 / general python formatting in the pyproject.toml, there is a bit of duplicated code now that needs to be trimmed down and consolidated (by @stvnksslr)

### Fixed
- *(#51)* another crack at improving the build system migration formats from python, ive chosen not to check for the actual path format yet as that will come in a later update (by @stvnksslr)

### Other
- chore(update readme): (by @stvnksslr)

### Contributors

* @stvnksslr
## [2025.4.0](https://github.com/stvnksslr/uv-migrator/compare/v2025.3.4...v2025.4.0) - 2025-01-15

### Added
- *(#53)* will attempt to init the uv project with any version information we can find in the previous packaging tool, if it doesnt find any will set the default python in the users path as is the default in uv (by @stvnksslr)
- *(#51)* adds new build system migration only supports poetry currently (by @stvnksslr)
- *(refactor)* alot of small bugs were cropping up from complex project setups that had lots of different files we were tracking, to make this easier to manage logic wise eveything has been broken into specific migration logic (by @stvnksslr)

### Fixed
- *(python version detection)* simplified logic to increase the chances of uv finding a python version within the range, ie 3.11 should find any installed 3.11 versions. (by @stvnksslr)

### Other
- chore(changing semver checks in releaseplz): (by @stvnksslr)

### Contributors

* @stvnksslr
## [2025.3.4](https://github.com/stvnksslr/uv-migrator/compare/v2025.3.3...v2025.3.4) - 2025-01-07

### Added
- *(poetry authors)* solves issue #49, support for migrating authors was only for setup.py but now supports poetry properly as well (by @stvnksslr)

### Other
- *(poetry indexes)* there was an order of operations bug that was filtering the indexes out before they could be migrated introduced in a recent version, this fixes it and also fixes #50 (by @stvnksslr)

### Contributors

* @stvnksslr
## [2025.3.3](https://github.com/stvnksslr/uv-migrator/compare/v2025.3.2...v2025.3.3) - 2025-01-03

### Other
- *(poetry)* poetry scripts should finally be fixed properly as tracked in issue #44 (by @stvnksslr)

### Contributors

* @stvnksslr
## [2025.3.2](https://github.com/stvnksslr/uv-migrator/compare/v2025.3.1...v2025.3.2) - 2025-01-01

### Other
- chore(formatting): (by @stvnksslr)
- bugfix(poetry scripts): (by @stvnksslr)

### Contributors

* @stvnksslr
## [2025.3.1](https://github.com/stvnksslr/uv-migrator/compare/v2025.3.0...v2025.3.1) - 2025-01-01

### Fixed
- *(clippy)* fixed some clippy warnings (by @stvnksslr)

### Other
- *(dependencies)* self_update released a needed bump .41 -> 42 (by @stvnksslr)

### Contributors

* @stvnksslr
## [2025.2.7](https://github.com/stvnksslr/uv-migrator/compare/v2025.2.6...v2025.2.7) - 2024-12-27

### Added
- feature - pipenv detection and migration ([#41](https://github.com/stvnksslr/uv-migrator/pull/41)) (by @stvnksslr)

### Contributors

* @stvnksslr
## [2025.2.6](https://github.com/stvnksslr/uv-migrator/compare/v2025.2.5...v2025.2.6) - 2024-12-26

### Other
- chore(more goreleaser troubleshooting): (by @stvnksslr)

### Contributors

* @stvnksslr
## [2025.2.5](https://github.com/stvnksslr/uv-migrator/compare/v2025.2.4...v2025.2.5) - 2024-12-26

### Other
- chore(more goreleaser troubleshooting): (by @stvnksslr)

### Contributors

* @stvnksslr
## [2025.2.4](https://github.com/stvnksslr/uv-migrator/compare/v2025.2.3...v2025.2.4) - 2024-12-26

### Other
- chore(rolling back): (by @stvnksslr)
- *(fixing release names)* lining up release names with rust defaults vs goreleaser defaults (by @stvnksslr)
- *(updating install.sh)* filename formats changed slightly with the move back to goreleaser (by @stvnksslr)

### Contributors

* @stvnksslr
## [2025.2.3](https://github.com/stvnksslr/uv-migrator/compare/v2025.2.2...v2025.2.3) - 2024-12-26

### Other
- wip(goreleaser): (by @stvnksslr)

### Contributors

* @stvnksslr
## [2025.2.2](https://github.com/stvnksslr/uv-migrator/compare/v2025.2.1...v2025.2.2) - 2024-12-26

### Other
- wip(goreleaser): (by @stvnksslr)
- wip(goreleaser): (by @stvnksslr)

### Contributors

* @stvnksslr
## [2025.2.1](https://github.com/stvnksslr/uv-migrator/compare/v2025.2.0...v2025.2.1) - 2024-12-26

### Other
- *(release workflow)* goreleaser(rust support) + release plz (by @stvnksslr)
- *(release workflow)* goreleaser(rust support) + release plz (by @stvnksslr)
- *(readme)* formatting (by @stvnksslr)

### Contributors

* @stvnksslr
## [2025.1.0](https://github.com/stvnksslr/uv-migrator/compare/v2025.0.0...v2025.1.0) - 2024-12-15

### Added
- *(install.sh)* simplified install script further so that its only concerned with if the chose folder exists, and if its in path. it will not create or modify the users path in anyway (by @stvnksslr)

### Other
- *(readme, --help)* reordering options to make more sense for common usage (by @stvnksslr)
- *(readme)* condensing readme (by @stvnksslr)
- chore(fix version): (by @stvnksslr)
- chore(deps + build opts): (by @stvnksslr)

### Contributors

* @stvnksslr
## [2025.0.0](https://github.com/stvnksslr/uv-migrator/compare/v2024.8.2...v2025.0.0) - 2024-12-14

### Added
- *(install.sh)* simplified and condensed logic as much as possible so the script is easily auditable (by @stvnksslr)
- *(merge dep groups)* since uv follows its own behavoir and treats additional groups as optional added the ability to merge dep groups into -dev for ease of use (by @stvnksslr)

### Fixed
- *(release)* messing with security settings (by @stvnksslr)
- *(zizmor)* used zizmor to find and fix all security related warnings (by @stvnksslr)
- *(readme)* install.sh link was wring (by @stvnksslr)

### Other
- chore(readme update + install script in repo): (by @stvnksslr)

### Contributors

* @stvnksslr
## [2024.8.2](https://github.com/stvnksslr/uv-migrator/compare/v2024.8.1...v2024.8.2) - 2024-12-10

### Fixed
- *(pyproject)* existing description and version fields were not being carried over correctly, this has been fixed (by @stvnksslr)

### Other
- chore(naming fixes): (by @stvnksslr)
- chore(naming for ci workflow): (by @stvnksslr)
- chore(naming for ci workflow): (by @stvnksslr)
- chore(rust format): (by @stvnksslr)
- *(doc strings)* added docstrings for tests (by @stvnksslr)

### Contributors

* @stvnksslr
## [2024.8.1](https://github.com/stvnksslr/uv-migrator/compare/v2024.8.0...v2024.8.1) - 2024-12-02

### Fixed
- *(tests)* copy paste error with the tests, fixed and the tests correctly get picked up (by @stvnksslr)

### Contributors

* @stvnksslr
## [2024.8.0](https://github.com/stvnksslr/uv-migrator/compare/v2024.7.4...v2024.8.0) - 2024-12-02

### Added
- *(rollback tests)* basic tests for the rollback feature (by @stvnksslr)
- *(rollback)* when an error migrating is throw, revert back to an actionable state, this allows for better feedback loop in trying to work on projects that may have dependencies that are in conflict which is allowed by pip (by @stvnksslr)

### Fixed
- *(linting)* clippy --fix (by @stvnksslr)
- *(linting)* clippy --fix (by @stvnksslr)

### Contributors

* @stvnksslr
## [2024.7.4](https://github.com/stvnksslr/uv-migrator/compare/v2024.7.3...v2024.7.4) - 2024-11-20

### Added
- feat(easy install option): (by @stvnksslr)

### Other
- chore(readme): (by @stvnksslr)

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

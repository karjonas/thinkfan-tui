
# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [0.3.1] - 2025-12-23

### Changed
- Use sudo instead of pkexec for chown
- Print full chown error if present

### Fixed
- Fix mismatched_lifetime_syntaxes warning
- Fix typos

## [0.3.0] - 2025-08-30

### Changed

- Verify that thinkpad_acpi module is loaded
- Add help popup window
- Add a shortcut (S key) to sort the inputs by temperature
- Make the Temperatures view scrollable

## [0.2.0] - 2025-06-18

### Changed

- Use colored graphs for temperature
- Exit on insufficient permissions
- Handle errors on read/write failures

## [0.1.1] - 2025-01-10

### Changed

- Cargo: pin dependency versions.

### Fixed

- Align Fan Info lines.
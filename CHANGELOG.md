# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.11] - 2026-06-12

### Fixed

- nmcli parsing on non English locales by [Daizygod](https://github.com/Daizygod)

## [0.1.10] - 2026-06-08

### Fixed

- KDE Plasma / KWin window sizing and override issues caused by restoring stale window geometry by [Gravitai-LAustin](https://github.com/Gravitai-LAustin)

## [0.1.9] - 2026-05-30

### Added

- Import profile by [xTITUSMAXIMUSX](https://github.com/xTITUSMAXIMUSX)
- Export profile by [xTITUSMAXIMUSX](https://github.com/xTITUSMAXIMUSX)
- Allow only one instance of wireguard-gui
- Save window position when closing and opening the app
- Use nmcli if available by default

### Fixed

- Snap compatibility

## [0.1.8] - 2025-10-27

### Added

- Splash screen when launching the application
- Default env variable to avoid gl failures

### Update

- Dependencies to nextjs 16 and tauri 2.9

## [0.1.7] - 2025-08-13

### Fix

- Quit application from the tray icon
- Crash when trying to get the ip address

### Update

- Dependencies

## [0.1.6] - 2024-10-14

### Added

- Upgrade to tauri v2
- Sort profiles by name case insensitive

## [0.1.5] - 2024-09-05

### Added

- Sort profiles by name
- Search bar to filter profiles by name

## [0.1.4] - 2024-09-02

### Changed

- Switch from zenity to polkit for authentication by [@CD11b](https://github.com/CD11b)

## [0.1.3] - 2024-08-04

### Fixed

- Close modal when creating a new profile

## Added

- Show current version in the header

## [0.1.2] - 2024-07-07

### Added

- Better error handling

## [0.1.1] - 2024-02-25

### Changed

- Icon instead of dropdown menu on profile table to delete, edit and connect

## [0.1.0] - 2024-01-30

### Added

- List profile
- Create profile
- Delete Profile
- Connect profile
- Disconnect

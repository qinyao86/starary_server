# Project Directory Layout

## Tracked source and packaging definitions

```text
starary-server/
|- src/                         Rust server source
|- admin-ui/src/                React administration source
|- desktop/                     Tauri desktop shell source and NSIS config
|- scripts/                     Development and release build scripts
|- packaging/windows/           Files copied into the Windows package
|- binaries/windows-x64/        Tracked Windows runtimes (Git LFS)
|- docs/                        Architecture and maintenance documentation
|- Cargo.toml / Cargo.lock      Rust dependency definitions
`- admin-ui/package*.json       Frontend dependency definitions
```

## Untracked inputs and generated output

```text
vendor/postgresql/archives/     Original third-party PostgreSQL ZIP
target/                         All generated build and release output
  |- build/frontend/            Production frontend builds
  |- build/desktop/             Production backend and desktop builds
  |- build-dev/frontend/        Development frontend builds
  |- build-dev/desktop/         Development backend and desktop builds
  `- release/windows-x64/       Final distributable artifacts
admin-ui/node_modules/          Installed frontend development dependencies
.dev/                           Local development data and storage
target/release/windows-x64/      Final distributable artifacts only
  |- Starary-Server_0.1.0_windows-x64-setup.exe
  |- Starary-Server_0.1.0_windows-x64-portable.zip
  `- SHA256SUMS.txt
```

All directories in the second list are ignored by Git. Never run the server
from `artifacts/` and never put initialized data directories there. Installed
desktop builds keep mutable data in `%ProgramData%\Starary Server`; program
files and the bundled PostgreSQL runtime remain read-only in the install folder.

## Common commands

Run the server from source with the development `.env` configuration:

```powershell
cargo run --manifest-path .\Cargo.toml
```

Build the Windows desktop installer:

```powershell
npm run release:windows
```

The NSIS installer lets the user choose a current-user or all-users install.
The Tauri shell starts the server without a console window and enforces a single
running server for the machine data directory.

Build the headless Windows portable release for a dedicated or cloud server:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\build-windows-portable.ps1
```

Clear Rust compiler output when disk space is more important than incremental
build speed:

```powershell
cargo clean
```

Normal packaging reads the minimized PostgreSQL runtime from
`binaries/windows-x64/postgresql/`; those files travel with the source through
Git LFS. The original PostgreSQL archive is not retained. It is only downloaded
when intentionally regenerating the runtime for an upgrade, and its SHA-256 is
pinned in `packaging/postgresql-windows-x64.json`.

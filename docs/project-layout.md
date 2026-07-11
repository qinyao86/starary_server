# Project Directory Layout

## Tracked source and packaging definitions

```text
madlibrary-server/
|- src/                         Rust server source
|- admin-ui/src/                React administration source
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
target/                         Rust output and temporary package assembly
  |- debug/                     `cargo build` and `cargo run`
  |- release/                   `cargo build --release`
  `- package/windows-x64/       Temporary portable package staging
admin-ui/node_modules/          Installed frontend development dependencies
admin-ui/dist/                  Vite production build output
.dev/                           Local development data and storage
release/                        Final distributable artifacts only
  `- madlibrary-server-windows-x64.zip
```

All directories in the second list are ignored by Git. `target/package` is
deleted after a successful portable build. `release/` must contain only files
that are ready to distribute; never run the server directly from it and never
put initialized `data/` directories there.

## Common commands

Run the server from source with the development `.env` configuration:

```powershell
cargo run --manifest-path .\Cargo.toml
```

Build the Windows portable release:

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

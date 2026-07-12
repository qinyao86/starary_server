# Mad Library Team Server

`E:\AI\codex\madlibrary_server` is the standalone Mad Library team server repository. It is extracted from `E:\AI\codex\madlibrary\server` so the server can be maintained separately from the desktop client.

## What Is Included

- Rust HTTP server bootstrap and API routes.
- PostgreSQL connection pool and idempotent startup schema migration.
- JWT auth, first-owner setup flow, and server user management.
- Team library, member, shared storage root, asset count, and activity endpoints.
- `admin-ui/` React + Vite admin console with dark theme by default.
- PowerShell development helpers and optional Docker Compose PostgreSQL config.

## Run Locally

Recommended path with installed PostgreSQL:

1. Create a PostgreSQL database with your local installed PostgreSQL.
2. Copy `.env.example` to `.env`.
3. Update `MADLIBRARY_DATABASE_URL` in `.env`.
4. Install and build the admin UI:

```powershell
cd .\admin-ui
npm install
npx vite build
cd ..
```

5. Start the server:

```powershell
cargo run --manifest-path .\Cargo.toml
```

Then open:

```text
http://127.0.0.1:3789/admin/
```

The server listens on `0.0.0.0:3789` by default so the admin UI and client API
are available to other computers on the LAN. Use the LAN address shown in the
server Settings page, for example `http://192.168.0.51:3789/admin/`. The
desktop shell itself always connects through `127.0.0.1`.

On Windows, set the host network profile to **Private** and allow inbound TCP
port `3789` for Private and Domain profiles. Do not expose the port on a Public
network. Bundled PostgreSQL remains private on `127.0.0.1:54329` and must not
be added to the firewall.

The first Owner account can only be created from the server host. After initial
setup, authorized administrators may use the browser UI from any permitted LAN
computer.

Optional Docker Desktop path:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\dev-up.ps1
```

Stop the development PostgreSQL container:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\dev-down.ps1
```

Check development status:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\dev-status.ps1
```

## Windows Desktop Release

The desktop installer is the default deployment for a LAN host. It provides a
Tauri management window, starts the server and bundled PostgreSQL without a
console window, and prevents duplicate service instances on the same machine.
The NSIS installer supports both current-user and all-users installation.

```powershell
npm run release:windows
```

The installer and SHA-256 checksum are written to
`artifacts/windows-x64/`. Mutable server data lives in
`%ProgramData%\Mad Library Server`, outside the application install directory.

## Windows Portable Release

The headless Windows x64 release bundles a private PostgreSQL runtime. End users do not
need to install PostgreSQL, Docker, Node.js, or Rust.

The minimized PostgreSQL runtime is committed under
`binaries/windows-x64/postgresql/` with Git LFS. Build the portable package:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\build-windows-portable.ps1
```

Normal builds do not download or extract PostgreSQL. The original archive is
only required when intentionally regenerating the tracked runtime with
`scripts/prepare-postgresql-runtime.ps1`.

The output is `artifacts/windows-x64/Mad-Library-Server_0.1.0_windows-x64-portable.zip`. Extract it and
double-click `start-server.cmd`. On first startup the server generates private
local credentials, initializes PostgreSQL, creates the application database,
runs schema migrations, and opens the admin console.

Persistent files live under the package's `data/` directory. Replacing the
application and PostgreSQL runtime during an upgrade must not replace `data/`.
PostgreSQL only listens on `127.0.0.1:54329`.

Database startup is controlled by `MADLIBRARY_POSTGRES_MODE=auto|bundled|external`.
The default `auto` mode starts packaged PostgreSQL only when
`MADLIBRARY_DATABASE_URL` is not set.

## Repository Layout

Source, downloaded inputs, development output, packaging intermediates, and
release artifacts have separate directories. See
[`docs/project-layout.md`](docs/project-layout.md) before adding generated or
third-party files.

## Important Notes

- `.env` is read from the application root.
- Relative paths in `.env`, such as `.dev/storage`, resolve from the application root.
- `MADLIBRARY_ADMIN_ASSETS_DIR` defaults to `admin-ui/dist`.
- `scripts/dev-up.ps1` builds `admin-ui` before starting the Rust server.
- Startup migrations are intentionally idempotent. Existing installed PostgreSQL databases are upgraded in place when columns are added.
- An explicit `MADLIBRARY_DATABASE_URL` disables bundled PostgreSQL and keeps
  external-database development and deployment available.

## Current First-Version Scope

The server console can currently:

- Create the first Owner account, then optionally guide the Owner through
  creating the first team library.
- Log in with JWT-backed sessions.
- Create, edit, activate, deactivate, and reset passwords for server users.
- Create, edit, soft-delete, and summarize team libraries.
- Reserve one exclusive final storage directory per library while allowing
  multiple libraries to share the same storage connection.
- Add and remove library members while preserving at least one manager-capable member.
- Create, edit, enable, disable, and delete shared storage roots when they are not referenced by active assets.
- Show global activity and per-library activity with readable actors and targets.
- Show health, database, storage, deployment, settings, and server info panels.

Still intentionally out of scope for this first server console version:

- Browsing or editing individual assets.
- Thumbnail/import workers.
- Backup execution.
- Full external identity provider integration.
- Production-grade invitation emails.

## Admin UI

Run the admin UI separately during frontend work:

```powershell
cd .\admin-ui
npm install
npm run dev
```

Open:

```text
http://127.0.0.1:5179/admin/
```

The Vite dev server proxies `/api` and `/health` to `http://127.0.0.1:3789`.

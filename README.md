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

## Important Notes

- `.env` is read from the repository root.
- Relative paths in `.env`, such as `.dev/storage`, resolve from the repository root.
- `MADLIBRARY_ADMIN_ASSETS_DIR` defaults to `admin-ui/dist`.
- `scripts/dev-up.ps1` builds `admin-ui` before starting the Rust server.
- Startup migrations are intentionally idempotent. Existing installed PostgreSQL databases are upgraded in place when columns are added.

## Current First-Version Scope

The server console can currently:

- Create the first Owner account and default team library.
- Log in with JWT-backed sessions.
- Create, edit, activate, deactivate, and reset passwords for server users.
- Create, edit, soft-delete, and summarize team libraries.
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

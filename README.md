# Mad Library Team Server

`E:\AI\codex\madlibrary_server` is the standalone Mad Library team server repository. It is extracted from `E:\AI\codex\madlibrary\server` so the server can be maintained separately from the desktop client.

## What Is Included

- Rust HTTP server bootstrap and API routes.
- PostgreSQL connection pool and prototype schema migration.
- JWT auth, setup flow, admin user and library management endpoints.
- Shared storage root and team asset/activity endpoints.
- `admin-ui/` React + Vite admin console.
- PowerShell development helpers and Docker Compose PostgreSQL config.

## Run Locally

Fast path with Docker Desktop:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\dev-up.ps1
```

Then open:

```text
http://127.0.0.1:3789/admin
```

Stop the development PostgreSQL container:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\dev-down.ps1
```

Check development status:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\dev-status.ps1
```

Manual path:

1. Copy `.env.example` to `.env`.
2. Update `MADLIBRARY_DATABASE_URL`.
3. Build the admin UI:

```powershell
cd .\admin-ui
npm install
npm run build
```

4. Run the server from the repository root:

```powershell
cargo run --manifest-path .\Cargo.toml
```

## Important Notes

- `.env` is read from the repository root.
- Relative paths in `.env`, such as `.dev/storage`, resolve from the repository root.
- `MADLIBRARY_ADMIN_ASSETS_DIR` defaults to `admin-ui/dist`.
- `scripts/dev-up.ps1` builds `admin-ui` before starting the Rust server.

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

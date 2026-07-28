# Agent Notes

## Build Layout

Use `target/` as the shared build root.

- `target/build/frontend/` for frontend build output.
- `target/build/desktop/` for backend and desktop shell build output.
- `target/build-dev/frontend/` for development frontend output.
- `target/build-dev/desktop/` for development backend and desktop shell output.
- `target/release/windows-x64/` for the final installer and checksum.

Do not write new release artifacts to `artifacts/`, `desktop/target/`, or
`target/core/`. Those paths are legacy layout leftovers and should be removed
when they are no longer needed.

## Admin UI Build Sync

When changing `admin-ui` and verifying through the desktop-managed server at
`http://127.0.0.1:3789/admin/`, do not stop after `npm run build`.

Vite writes the production build to:

```text
target/build/frontend/admin-ui/
```

The desktop development runtime serves a copied bundle from:

```text
target/build-dev/desktop/runtime/admin-ui/
```

For a frontend change that must be visible in the running development desktop
and admin server, build directly into the development frontend directory and
run the shared prep script in development mode:

```powershell
$env:STARARY_ADMIN_UI_OUT_DIR = "$PWD\target\build-dev\frontend\admin-ui"
Push-Location .\admin-ui
npm run build
Pop-Location
powershell -ExecutionPolicy Bypass -File .\scripts\prepare-desktop-bundle.ps1 -Development
```

Alternatively, run the full desktop development flow:

```powershell
npm run desktop:dev
```

The server control-center shell uses `http://127.0.0.1:14310` in development.
`tauri dev` starts that static Vite server through `beforeDevCommand`; do not run
the debug executable directly because it will not have a development page to load.
The `scripts/desktop-dev-server.mjs` wrapper owns the Vite child and watches the
desktop PID marker. A full tray-menu exit must remove the marker and release
port `14310`; closing the titlebar to the tray intentionally keeps both alive.

That script builds `admin-ui`, prepares `target/build-dev/desktop/runtime/`,
and starts Tauri. If the page still looks stale after the sync, refresh the
browser or restart the desktop window so it reloads the new assets.

## Product Identity

- The server package and executable are `starary-server`; the desktop package is `starary-server-desktop`.
- Runtime configuration uses only `STARARY_*` environment variables and the desktop control header is `X-Starary-Control-Token`.
- Machine data lives under `%ProgramData%\Starary Server` on Windows and `.starary-server` on other platforms.
- Team-library internal files live under `.starary/`; do not add `.madlibrary` runtime compatibility branches.
- The bundled PostgreSQL defaults are database `starary_team` and user `starary`.
- The `madlibrary-quick-hash-v2-sampled-256k` byte marker is a stable cross-client/server hash algorithm namespace. It is intentionally not a product identity and must not be renamed without a standalone full rehash migration.

# Agent Notes

## Admin UI Build Sync

When changing `admin-ui` and verifying through the desktop-managed server at
`http://127.0.0.1:3789/admin/`, do not stop after `npm run build`.

Vite writes the production build to:

```text
admin-ui/dist/
```

The desktop development runtime serves a copied bundle from:

```text
target/desktop-runtime/admin-ui/
```

After every frontend build that should be visible in the desktop/admin server
runtime, sync the files:

```powershell
robocopy admin-ui\dist target\desktop-runtime\admin-ui /E /R:0 /W:0 /NFL /NDL /NJH /NJS /NP
if ($LASTEXITCODE -gt 7) { exit $LASTEXITCODE }
```

Alternatively, run the full desktop development flow:

```powershell
npm run desktop:dev
```

That script builds `admin-ui`, prepares `target/desktop-runtime/`, and starts
Tauri. If the page still looks stale after the sync, refresh the browser or
restart the desktop window so it reloads the new hashed Vite assets.

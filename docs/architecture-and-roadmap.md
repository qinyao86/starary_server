# Mad Library Server Architecture and Roadmap

## Product Direction

Mad Library Server is proprietary commercial software licensed by team seats.
The same Rust server core supports two deployment forms without exposing its
database directly to clients.

## Deployment Forms

### Windows LAN appliance

- One Windows x64 portable package.
- `madlibrary-server.exe` serves the API and built React administration UI.
- A minimal PostgreSQL 16 runtime is bundled and managed by the server process.
- Local disks and SMB shares are available as filesystem storage roots.
- Persistent data is isolated under `data/` so application upgrades do not
  replace database or asset data.

### Internet or cloud server

- Linux OCI container is the preferred artifact; a native systemd package can
  be added when required.
- External managed PostgreSQL is preferred.
- S3-compatible object storage is preferred for asset files.
- TLS terminates at a reverse proxy or cloud gateway.
- The initial commercial model is one isolated server instance per team.

Docker is not a dependency of the Windows LAN package. It is an optional cloud
deployment mechanism and remains useful for local development.

## PostgreSQL Modes

`MADLIBRARY_POSTGRES_MODE` controls database startup:

- `auto` (default): use `MADLIBRARY_DATABASE_URL` when set; otherwise start the
  bundled runtime.
- `bundled`: require and start the PostgreSQL runtime beside the executable.
- `external`: never start PostgreSQL; require `MADLIBRARY_DATABASE_URL`.

PostgreSQL listens only on `127.0.0.1` in bundled mode. The server generates a
random database password and JWT secret on first startup and stores them under
`data/config/`. Application schema migrations remain automatic and idempotent.

## Storage Model

Filesystem storage covers server-local disks, SMB, and NFS. Desktop clients use
platform path aliases so a shared asset can be opened like a local file.

Object storage uses an S3-compatible provider. A desktop synchronization agent
downloads cloud-only files into a configured local sync directory before they
are opened. The sync model must represent cloud-only, downloading, local,
uploading, synchronized, conflict, and failed states. A kernel filesystem
driver is intentionally deferred.

Storage connections describe reusable physical roots such as a disk folder,
SMB share, or object-storage bucket. Each library receives exactly one exclusive
final directory below a connection. Two libraries may share a connection, but
their final directories must not be equal, parents, or children of each other.
Soft-deleted and disabled libraries retain their directory reservation.

An empty library may change its binding directly. Once an asset has referenced
the library storage, the binding is permanently marked as in use. Further
changes must use a storage migration workflow that copies and verifies files,
atomically switches the binding, and retains the old location for rollback.

## Commercial Licensing

- Keep source repositories private and distribute proprietary binaries.
- Enforce signed team licenses and seat limits on the server.
- Support offline signed licenses for LAN installations and online refresh for
  internet deployments.
- Keep required PostgreSQL and third-party license notices in every package.
- Audit Rust and npm dependency licenses before commercial releases.

## Delivery Phases

1. Complete and verify the minimal Windows PostgreSQL bundle and one-click
   startup/shutdown lifecycle.
2. Add backup, restore, upgrade, diagnostics, and optional Windows Service
   installation.
3. Add signed team licensing and seat enforcement.
4. Produce the Linux cloud image with external PostgreSQL and TLS deployment
   documentation.
5. Implement S3-compatible storage and transfer workers.
6. Implement the desktop sync directory, conflict handling, and local-open
   workflow.

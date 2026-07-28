# Starary Server Architecture and Roadmap

## Product Direction

Starary Server is proprietary commercial software licensed by team seats.
The same Rust server core supports two deployment forms without exposing its
database directly to clients.

## Deployment Forms

### Windows LAN appliance

- One Windows x64 installer with a small local control center.
- `starary-server.exe` serves the API and built React administration UI.
- A minimal PostgreSQL 16 runtime is bundled and managed by the server process.
- Local disks and SMB shares are available as filesystem storage roots.
- Persistent data is isolated under `data/` so application upgrades do not
  replace database or asset data.

The Tauri application is a local service control center, not an embedded copy
of the administration UI. It starts, stops, restarts, and diagnoses the managed
server, while all library, storage, user, backup, and settings workflows remain
in the browser administration UI. Closing its window hides it to the system
tray. Exiting from the tray closes only the control center and leaves the
server running; stopping the server is always a separate explicit action.

The control center and managed server use a persisted installation instance ID
and local control token under the machine data directory. A control center may
only manage a server that returns the matching identity over a loopback-only
control endpoint. It never claims or stops a process merely because it uses the
configured port or has a matching executable name. The control center itself
is single-instance, and the server keeps its existing per-data-directory
instance lock.

The first release still launches the managed server as a detached background
process. Installing the same server core as a Windows Service is the next
hardening step for startup before user sign-in, service recovery policies, and
multi-user hosts; it does not change the browser administration architecture.

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

`STARARY_POSTGRES_MODE` controls database startup:

- `auto` (default): use `STARARY_DATABASE_URL` when set; otherwise start the
  bundled runtime.
- `bundled`: require and start the PostgreSQL runtime beside the executable.
- `external`: never start PostgreSQL; require `STARARY_DATABASE_URL`.

PostgreSQL listens only on `127.0.0.1` in bundled mode. The server generates a
random database password and JWT secret on first startup and stores them under
`data/config/`. Application schema migrations remain automatic and idempotent.

The bundled runtime creates database `starary_team` with database user
`starary`. Windows machine data is rooted at `%ProgramData%\Starary Server`,
and team-library derived media is stored under each library's `.starary/`
directory. Legacy Mad Library storage is not read by the runtime; any future
data transfer must be implemented as a standalone removable migration tool.

The HTTP service listens on `0.0.0.0` by default and exposes both the client API
and browser administration UI on the configured server port. The desktop
control center opens that UI in the system browser rather than embedding it.
Initial Owner creation is restricted to loopback connections; subsequent
administration is available over authenticated LAN browser sessions. Production
internet deployments must terminate TLS at a reverse proxy and should not
expose the bundled PostgreSQL port.

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

## Account Lifecycle

Team access is invite-first rather than open registration. An owner or
administrator creates an invitation with the user's email, initial server role,
and optional library assignments. The recipient accepts the invitation and sets
their own password. LAN deployments can expose the invitation as a copyable
one-time link or code without requiring an email service; internet deployments
may send the same link by email.

Servers may optionally accept join requests. A request never creates an active
account, consumes a licensed seat, or grants library access until an
administrator approves it. Approval can happen asynchronously, so an
administrator does not need to remain online.

Signed-in users can change their own password after confirming the current
password. Initially, forgotten passwords are reset by an administrator using a
temporary one-time credential. Email-based password recovery is added only for
deployments with a configured mail provider. Invitations, approvals, password
changes, resets, and account activation changes are audit events.

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

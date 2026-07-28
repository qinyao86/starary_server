Starary Server - Windows x64 portable package

START
  Double-click start-server.cmd.
  The management console opens at http://127.0.0.1:3789/admin/ by default.
  Owners and administrators can change the port in Settings. Restart the
  executable after saving; start-server.cmd reads the saved port automatically.

STOP
  Press Ctrl+C in the server window and wait for PostgreSQL to stop.

DATA
  All persistent files are under the data directory:
    data\postgresql  Database files
    data\storage     Managed asset storage
    data\config      Generated local credentials
    data\logs        PostgreSQL logs

  Back up the whole data directory. Do not replace or delete it during an
  application upgrade.

REQUIREMENTS
  No separate PostgreSQL, Docker, Node.js, or Rust installation is required.

DATABASE MODE
  The default mode is automatic: packaged PostgreSQL starts unless an external
  MADLIBRARY_DATABASE_URL is configured in a .env file beside the executable.

  Optional .env setting:
    MADLIBRARY_POSTGRES_MODE=auto      Default behavior
    MADLIBRARY_POSTGRES_MODE=bundled   Require packaged PostgreSQL
    MADLIBRARY_POSTGRES_MODE=external  Require MADLIBRARY_DATABASE_URL

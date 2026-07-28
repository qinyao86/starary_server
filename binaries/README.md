# Bundled runtimes

This directory contains minimized third-party runtimes used directly by release
packaging. Its layout follows the main Starary desktop repository:

```text
binaries/<platform>/<component>/
```

`windows-x64/postgresql/` is the complete PostgreSQL runtime required by the
Windows server package. Executables and DLLs are tracked with Git LFS; small
support files use regular Git. Normal builds copy these files directly and do
not read or download the original PostgreSQL archive.

The original archive is only needed when upgrading or regenerating this
runtime. See `scripts/prepare-postgresql-runtime.ps1` and
`packaging/postgresql-windows-x64.json`.

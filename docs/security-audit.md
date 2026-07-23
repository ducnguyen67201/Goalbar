# Security audit notes

## RUSTSEC-2023-0071

`cargo audit` sees `rsa 0.9.10` in `Cargo.lock` through SQLx's optional macro/database metadata. The application disables SQLx default features and enables only the Tokio, SQLite, Chrono, and UUID features. `cargo tree --target all -i rsa` reports no dependency path, so the affected RSA implementation is not compiled or reachable in Tagline.

The `audit:rust` command therefore ignores only `RUSTSEC-2023-0071`. Remove this exception if RSA becomes reachable, SQLx changes its lock metadata, or a fixed RSA release becomes available. All other RustSec vulnerabilities still fail the command.

RustSec also reports non-failing warnings in target-universal dependency metadata: unmaintained GTK3-era crates, unmaintained transitive Unicode crates, and `RUSTSEC-2024-0429` for `glib 0.18.5`. The GTK/glib path belongs to Tauri's current Linux webview stack and is not compiled into the validated macOS bundle, but it must be treated as an upstream Linux release risk rather than dismissed as maintenance noise. Re-run the audit for every platform release and upgrade the Tauri Linux stack as soon as its supported webview dependencies move off these versions.

# Security audit notes

## RUSTSEC-2023-0071

`cargo audit` sees `rsa 0.9.10` in `Cargo.lock` through SQLx's optional macro/database metadata. The application disables SQLx default features and enables only the Tokio, SQLite, Chrono, and UUID features. `cargo tree --target all -i rsa` reports no dependency path, so the affected RSA implementation is not compiled or reachable in Tagline.

The `audit:rust` command therefore ignores only `RUSTSEC-2023-0071`. Remove this exception if RSA becomes reachable, SQLx changes its lock metadata, or a fixed RSA release becomes available. All other RustSec vulnerabilities still fail the command.

RustSec also reports non-failing maintenance warnings for GTK3-era dependencies used by Tauri's Linux webview stack. Those are tracked upstream and do not affect the macOS bundle produced by the current release gate.

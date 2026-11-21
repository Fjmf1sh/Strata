# Strata

Strata is a native Rust launcher project built from the Modrinth open-source app codebase. The custom desktop UI uses `winit` and `softbuffer`; the native backend reuses the Theseus Rust package in `packages/app-lib`.

The native UI is an active port and is not yet feature-complete or visually identical to Modrinth. The upstream frontend remains in this workspace as a behavior and design reference. See [NATIVE_MIGRATION_PLAN.md](NATIVE_MIGRATION_PLAN.md) for current coverage and remaining work.

## Build

Install the Rust toolchain selected by `rust-toolchain.toml`.

```sh
cargo build --release -p native-ui
```

Linux builds use Winit's X11 and Wayland backends. Install the X11 and Wayland development libraries listed in `.github/workflows/native-ui-cross-platform.yml` before building on Linux.

The requested 60 MiB process-memory ceiling is still a target and has not yet been verified with runtime profiling. Image caches and decode concurrency have explicit limits.

## Horizon

Horizon support is planned as a separate game target. Java mod JARs are not compatible with Horizon; supported content needs to use Horizon's own import and mod workflows.

## Licensing

Strata's native launcher additions use GPL-3.0-only; see [LICENSE](LICENSE). The upstream packages and assets retained in this workspace remain subject to their package-specific licenses and notices in [COPYING.md](COPYING.md).

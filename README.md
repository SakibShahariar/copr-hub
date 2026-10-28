# Copr Hub

Copr Hub is a native Linux desktop application for discovering Fedora COPR projects, inspecting packages, and managing COPR repositories. It is built with **Rust, GTK4, and libadwaita**, following GNOME conventions.

> **Development status:** The app has GNOME-style top navigation and its first live feature: searching the COPR project catalogue. Local repository management, package installation, and detailed project views are still in development.

## Current functionality

- Search COPR projects through the public COPR API.
- Display project names, owners, descriptions, and optional homepage links.
- Show empty-result, network-error, HTTP-error, and timeout feedback.
- Keep network requests off the GTK main thread.
- Use native GTK/libadwaita widgets and the active system theme, so Matugen palette changes can flow through the interface.

## Planned features

- Open full project details, build status, Fedora targets, and package lists.
- Inspect and manage enabled COPR repositories.
- Preview package-manager actions before making system changes.
- Show installed packages and recent operations.
- Integrate with Fedora without running the entire application as root.

## Technology

- Rust
- GTK4
- libadwaita
- COPR API v3

## Build on Fedora

Install the native development dependencies:

```bash
sudo dnf install gcc pkgconf-pkg-config gtk4-devel libadwaita-devel
```

Install Rust using [rustup](https://rustup.rs/), then run:

```bash
cargo fmt --all -- --check
cargo check
cargo run
```

For a strict local lint pass:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

## Project identity

- Display name: **Copr Hub**
- Application ID: `io.github.SakibShahariar.CoprHub`
- Repository: <https://github.com/SakibShahariar/copr-hub>

## License

Copr Hub is licensed under the MIT License. See [LICENSE](LICENSE).

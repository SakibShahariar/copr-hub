# Copr Hub

Copr Hub is a native Linux desktop application for discovering Fedora COPR projects, inspecting packages, and managing COPR repositories. It is built with **Rust, GTK4, and libadwaita**, following GNOME conventions.

> **Development status:** The application shell now has GNOME-style top navigation for Discover, Repositories, Installed, and Activity. COPR API search and local package-management operations are not implemented yet.

## Planned features

- Search and browse COPR projects and packages.
- View project details, owners, build status, and available packages.
- Inspect and manage enabled COPR repositories.
- Preview package-manager actions before making system changes.
- Handle network failures, empty results, and API limits clearly.
- Integrate with Fedora without running the entire application as root.
- Follow the active GTK/libadwaita theme so Matugen palette changes are reflected automatically.

## Technology

- Rust
- GTK4
- libadwaita

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

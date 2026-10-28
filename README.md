# Copr Hub

Copr Hub is a native Linux desktop application for discovering Fedora COPR projects, inspecting project metadata, and managing COPR repositories. It is built with **Rust, GTK4, and libadwaita**, following GNOME conventions.

## Current functionality

- Search the public COPR API for projects.
- Display project names, owners, descriptions, and optional homepage links.
- Open a project detail window from search results.
- Show useful feedback for empty searches, empty results, timeouts, and network/API failures.
- Run blocking network requests away from the GTK main thread.
- Use native GTK/libadwaita widgets and the active system theme. Accent colors are not hard-coded, so Matugen palette changes can be reflected by the existing GTK theme.

## Roadmap

- [ ] Fetch detailed project metadata, build targets, and package lists.
- [ ] Read and manage enabled COPR repositories.
- [ ] Show installed packages and their source repositories.
- [ ] Add transaction previews and safe package-management integration.
- [ ] Record recent operations and failures.
- [ ] Add saved projects and favorites.
- [ ] Improve accessibility, responsive layouts, and application packaging.

## Technology

- Rust
- GTK4
- libadwaita
- COPR API v3

## Build on Fedora

Install native development dependencies:

```bash
sudo dnf install gcc pkgconf-pkg-config gtk4-devel libadwaita-devel
```

Install Rust using [rustup](https://rustup.rs/), then run:

```bash
cargo fmt --all -- --check
cargo test
cargo check
cargo run
```

Run Clippy with warnings treated as errors:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

## Project identity

- Display name: **Copr Hub**
- Application ID: `io.github.SakibShahariar.CoprHub`
- Repository: <https://github.com/SakibShahariar/copr-hub>

## License

Copr Hub is licensed under the MIT License. See [LICENSE](LICENSE).

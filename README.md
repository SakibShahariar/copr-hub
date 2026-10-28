# Copr Hub

Copr Hub is a native Linux desktop application for discovering Fedora COPR projects, inspecting project metadata, and viewing configured COPR repositories. It is built with **Rust, GTK4, and libadwaita**, following GNOME conventions.

## Current functionality

- Search the public COPR API for projects.
- Display project names, owners, descriptions, and optional homepage links.
- Open project details with package names, build targets, reported build states, and package versions.
- Read enabled and disabled COPR repository definitions from `/etc/yum.repos.d` without modifying system files.
- Show feedback for empty results, timeouts, and network/API failures.
- Run blocking network and filesystem operations away from the GTK main thread.
- Use native GTK/libadwaita widgets and the active system theme. Accent colors are not hard-coded, allowing Matugen palette changes to flow through the existing GTK theme.
- Include a desktop entry, SVG icon, and AppStream metadata under `data/`.

Build status is reported by COPR and is **not** a security or trust assessment.

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

For a strict lint pass:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

## Roadmap

- [ ] Add explicit, confirmed repository enable/disable actions through the system package manager.
- [ ] Show installed packages and their source repositories.
- [ ] Add transaction previews and safe package-management integration.
- [ ] Record recent operations and failures.
- [ ] Add saved projects and favorites.
- [ ] Improve accessibility, responsive layouts, and application packaging.

## Project identity

- Display name: **Copr Hub**
- Application ID: `io.github.SakibShahariar.CoprHub`
- Repository: <https://github.com/SakibShahariar/copr-hub>

## License

Copr Hub is licensed under the MIT License. See [LICENSE](LICENSE).

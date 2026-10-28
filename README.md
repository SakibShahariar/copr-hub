# Copr Hub

Copr Hub is a native Linux desktop application for discovering Fedora COPR projects, inspecting packages, and managing COPR repositories. It is being built with **Rust, GTK4, and libadwaita** for a GNOME-friendly experience.

> **Development status:** Early scaffold. The current commit establishes the GTK application shell and navigation only; COPR API access, repository operations, and package installation are not implemented yet.

## Planned features

- Search and browse COPR projects and packages.
- View project details, owners, build status, and available packages.
- Inspect and manage enabled COPR repositories.
- Show safe, transparent installation commands and package-manager actions.
- Handle network failures, empty results, and API limits clearly.
- Integrate with the Fedora desktop without running the whole application as root.

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
cargo fmt --check
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

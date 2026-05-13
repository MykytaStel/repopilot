# Installation

RepoPilot is distributed through several channels so different developer workflows can use the same CLI.

## Recommended install paths

### Cargo

Use this when you already have Rust installed:

```bash
cargo install repopilot
```

Upgrade:

```bash
cargo install repopilot --force
```

### npm

Use this when you work mostly in JavaScript, TypeScript, React, or React Native projects:

```bash
npm install -g repopilot
```

Upgrade:

```bash
npm update -g repopilot
```

The npm package is a thin wrapper around the native Rust binary. During `postinstall`,
it downloads the matching GitHub Release artifact and verifies the `.sha256`
checksum before exposing the `repopilot` command.

Environment overrides:

```bash
REPOPILOT_SKIP_DOWNLOAD=1 npm install -g repopilot
REPOPILOT_BINARY_PATH=/path/to/repopilot repopilot --version
```

Use `REPOPILOT_BINARY_PATH` when your environment does not allow package installers
to download binaries.

### Homebrew

Use this on macOS or Linux when you prefer Homebrew-managed CLI tools:

```bash
brew tap mykytastel/repopilot
brew install repopilot
```

Upgrade:

```bash
brew update
brew upgrade repopilot
```

### Curl installer

Use this for quick Linux/macOS installs from GitHub Releases:

```bash
curl -fsSL https://raw.githubusercontent.com/MykytaStel/repopilot/main/install.sh | bash
```

By default, the script installs to:

```text
~/.local/bin/repopilot
```

Override the install directory:

```bash
INSTALL_DIR=/usr/local/bin curl -fsSL https://raw.githubusercontent.com/MykytaStel/repopilot/main/install.sh | sudo bash
```

The installer downloads the release archive and its `.sha256` file. Installation
fails closed if checksum verification cannot be completed.

### Build from source

```bash
git clone https://github.com/MykytaStel/repopilot.git
cd repopilot
cargo build --release
./target/release/repopilot --version
```

## Verify installation

After installing from any channel:

```bash
repopilot --version
repopilot doctor .
repopilot scan . --min-severity high
```

## Which channel should I choose?

| Channel | Best for |
|---|---|
| Cargo | Rust users and source-based installs |
| npm | JavaScript, TypeScript, React, and React Native developers |
| Homebrew | macOS/Linux users who prefer Homebrew-managed CLIs |
| Curl installer | Quick binary install from GitHub Releases |
| Source build | Contributors and maintainers |

## Troubleshooting

### `repopilot: command not found`

Make sure the install directory is on your `PATH`.

For the curl installer, add this to your shell profile if needed:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

### npm install cannot download the binary

Use a manually installed binary:

```bash
REPOPILOT_BINARY_PATH=/path/to/repopilot repopilot --version
```

Or skip the download during installation:

```bash
REPOPILOT_SKIP_DOWNLOAD=1 npm install -g repopilot
```

### Homebrew formula looks outdated

Refresh the tap and reinstall:

```bash
brew update
brew upgrade repopilot
```

# devcontainer-env

> Bridge devcontainers and the host environment — run host commands with devcontainer service environments and automatically rewrite container service URLs to host ports.

[![Test](https://github.com/devcontainer-env/devcontainer-env/actions/workflows/test.yml/badge.svg)](https://github.com/devcontainer-env/devcontainer-env/actions/workflows/test.yml)
[![Release](https://img.shields.io/github/v/release/devcontainer-env/devcontainer-env)](https://github.com/devcontainer-env/devcontainer-env/releases/latest)
[![Crate](https://img.shields.io/crates/v/devcontainer-env)](https://crates.io/crates/devcontainer-env)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

```bash
eval "$(devcontainer-env export)"
```

## Requirements

- [Rust](https://www.rust-lang.org/) 1.70+ (`rustc`, `cargo`)

**macOS (Homebrew):**

```bash
brew install rust
```

**Nix:**

```bash
nix profile install nixpkgs#rust
```

## Installation

### Cargo (Crates.io)

```bash
cargo install devcontainer-env
```

### Nix (recommended)

Run directly without installing:

```bash
nix run github:devcontainer-env/devcontainer-env -- exec -- your-command
```

Or install into your profile:

```bash
nix profile install github:devcontainer-env/devcontainer-env
```

## Usage

### Export Environment Variables

```bash
eval "$(devcontainer-env export)"
```

To make persistent, add to your `.envrc` file. Then run `direnv allow` to enable it.

### Run Commands

Execute commands with devcontainer environment variables:

```bash
devcontainer-env exec -- go test ./...
```

### Inspect Configuration

View parsed devcontainer configuration and service port mappings:

```bash
devcontainer-env inspect
```

### CLI Reference

```
Usage: devcontainer-env <COMMAND>

Commands:
  export    Export environment variables as shell statements
  exec      Execute command with devcontainer environment
  inspect   Display parsed devcontainer configuration
  help      Print help message

Options:
  -h, --help     Print help
  -V, --version  Print version
```

**devcontainer-env export** — Output environment variables from `containerEnv` as shell statements.

Only variables defined in the `containerEnv` section are exported to the host (not service configuration or service environment variables). See [Configuration](#configuration) for an example.

```bash
$ devcontainer-env export
```

```bash
export EXAMPLE_API_LOG_LEVEL=DEBUG
export EXAMPLE_API_LOG_PRETTY=true
export EXAMPLE_API_DATABASE_URL=postgres://vscode@127.0.0.1:32770/example-db?sslmode=disable
```

**devcontainer-env exec** — Run a command with devcontainer environment available:

```bash
devcontainer-env exec -- <COMMAND> [ARGS...]
```

The `--` separator is required. Everything after `--` is passed to the command.

**devcontainer-env inspect** — Parse and display the devcontainer configuration:

```bash
$ devcontainer-env inspect
```

```bash
Workspace: /Users/iamralch/Projects/github.com/example-org/example-api

Containers:
  default-co-bd5e8-postgres-1
    Image: postgres:18-bookworm
    Hosts: default-co-bd5e8-postgres-1, postgres, fe6eca554c95
    Ports: 5432 → 0.0.0.0:32770, 5432 → :::32770

  default-co-bd5e8-workspace-1, main
    Image: mcr.microsoft.com/devcontainers/base:noble
    Hosts: default-co-bd5e8-workspace-1, workspace, d93fdbd4f540

Environment:
  EXAMPLE_API_LOG_LEVEL = DEBUG
  EXAMPLE_API_LOG_PRETTY = true
  EXAMPLE_API_DATABASE_URL = postgres://vscode@127.0.0.1:32770/example-db?sslmode=disable
```

## Configuration

Configure your `.devcontainer/devcontainer.json` to define environment variables and services:

### devcontainer.json

```json
{
  "$schema": "https://raw.githubusercontent.com/devcontainers/spec/refs/heads/main/schemas/devContainer.base.schema.json",
  "name": "my-project",
  "service": "workspace",
  "dockerComposeFile": "docker-compose.yml",
  "workspaceFolder": "/home/vscode/workspace",
  "remoteUser": "vscode",
  "containerEnv": {
    "EXAMPLE_API_LOG_LEVEL": "DEBUG",
    "EXAMPLE_API_LOG_PRETTY": "true",
    "EXAMPLE_API_DATABASE_URL": "postgres://vscode@postgres:5432/example-db?sslmode=disable"
  }
}
```

### docker-compose.yml

```yaml
services:
  workspace:
    image: "mcr.microsoft.com/devcontainers/base:noble"
    command: sleep infinity

  postgres:
    image: postgres:18-bookworm
    restart: unless-stopped
    volumes:
      - postgres:/var/lib/postgres
    environment:
      POSTGRES_DB: my-project
      POSTGRES_USER: vscode
      POSTGRES_HOST_AUTH_METHOD: trust
    healthcheck:
      test: ["CMD-SHELL", "pg_isready"]
      interval: 1s
      timeout: 5s
      retries: 10
    ports:
      - 5432

volumes:
  postgres:
```

### Port Mapping

Use `ports: [<PORT>]` syntax (not `"HOST:PORT"`) to let Docker assign random available host ports. This prevents conflicts when running multiple devcontainers or projects simultaneously. `devcontainer-env export` automatically detects the Docker-assigned host port and makes it available in exported environment variables. Do not use `forwardPorts` in `devcontainer.json` — rely on `docker-compose.yml` port mapping instead.

## License

[MIT](LICENSE) — Copyright (c) 2025 devcontainer-env

<!-- markdownlint-disable-file MD013 -->

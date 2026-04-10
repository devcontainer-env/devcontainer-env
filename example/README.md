# Example Configuration

This directory demonstrates a complete, working devcontainer setup integrated with Nix and `devcontainer-env`.

## What This Example Shows

- **[`.devcontainer/devcontainer.json`](./.devcontainer/devcontainer.json)** — DevContainer configuration defining:
  - Workspace service (base image)
  - PostgreSQL service for local development
  - **`containerEnv`** — Environment variables that `devcontainer-env` exports to the host (more on this below)

- **[`.devcontainer/docker-compose.yml`](./.devcontainer/docker-compose.yml)** — Docker Compose definition for services:
  - `workspace` service running the development environment
  - `postgres` service with proper health checks and port mapping
  - Uses dynamic port mapping (`ports: [5432]`) so multiple projects don't conflict

- **[`flake.nix`](./flake.nix) / [`flake.lock`](./flake.lock)** — Nix flake showing how to:
  - Consume `devcontainer-env` from the main repository
  - Set up a development shell that automatically exports container environment variables

- **[`.github/workflows/ci.yml`](./.github/workflows/ci.yml)** — GitHub Actions workflow demonstrating:
  - Starting the DevContainer in CI with [`devcontainer-env/devcontainer-ci`](https://github.com/devcontainer-env/devcontainer-ci) action
  - Setting up Nix and caching
  - Running tests within `nix develop` with exported environment variables available

## How `nix develop` Works

When you run `nix develop` in this directory:

1. **Nix evaluates the flake** — It reads `flake.nix` and resolves all dependencies (including the devcontainer-env package)

2. **Creates an isolated dev shell** — A shell environment is created with the packages defined in `devShells.default`

3. **Runs the shellHook** — Before giving you the shell, it executes:

   ```bash
   eval "$(devcontainer-env export)"
   ```

4. **Exports container environment** — This command:
   - Reads your `.devcontainer/devcontainer.json`
   - Starts the Docker containers (if not already running)
   - Extracts environment variables like `EXAMPLE_API_DATABASE_URL` from the container
   - Sets them in your shell session

5. **You can now run commands** — Your host shell now has all container services available:
   ```bash
   $ nix develop
   $ psql $EXAMPLE_API_DATABASE_URL  # PostgreSQL is accessible
   $ env | grep EXAMPLE_API          # Container env vars are available
   ```

## Quick Start

1. Copy `.devcontainer/` to your project
2. Customize `devcontainer.json` and `docker-compose.yml` for your services
3. Copy `flake.nix` and update the service name, image, and environment variables
4. Initialize the devcontainer: `devcontainer up --workspace-folder .`
5. Run `nix develop` to enter the development shell with all services running

When you're done, exit the shell (`exit` or Ctrl+D) and the containers remain running. Run `nix develop` again to re-enter with the same environment.

## Key Features

- **Declarative** — All configuration in code (git-friendly)
- **Reproducible** — Same environment across machines with Nix
- **Automatic** — No manual container startup; `devcontainer up` and `nix develop` handle it
- **Integrated** — Host tools can talk to container services using exported environment variables

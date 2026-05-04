# Changelog

## [0.3.2](https://github.com/devcontainer-env/devcontainer-env/compare/v0.3.1...v0.3.2) (2026-05-04)


### Bug Fixes

* **github:** correct action versions in update.yml ([00cc3c3](https://github.com/devcontainer-env/devcontainer-env/commit/00cc3c32e0740879f765b48ec7f4b96119d87078))

## [0.3.1](https://github.com/devcontainer-env/devcontainer-env/compare/v0.3.0...v0.3.1) (2026-05-01)


### Bug Fixes

* **ci:** remove blank lines and bump macos runner to macos-15 ([2288b33](https://github.com/devcontainer-env/devcontainer-env/commit/2288b3395146179f2a62b6d7a3a035c4309bb8c3))

## [0.3.0](https://github.com/devcontainer-env/devcontainer-env/compare/v0.2.0...v0.3.0) (2026-05-01)


### Features

* **oci:** add no-op Docker client for running inside a container ([1ddee02](https://github.com/devcontainer-env/devcontainer-env/commit/1ddee02372fe1fcef1ac66c3b50087712c551975))


### Bug Fixes

* correct CI badge workflow filename ([08cc0eb](https://github.com/devcontainer-env/devcontainer-env/commit/08cc0eba8aff8b8ad93b9c1929610442e424c6f3))

## [0.2.0](https://github.com/devcontainer-env/devcontainer-env/compare/v0.1.0...v0.2.0) (2026-04-09)


### Features

* auto-detect export format from $SHELL environment variable ([23849c9](https://github.com/devcontainer-env/devcontainer-env/commit/23849c95ae06496b1da8e8460bf931c88bc11326))

## 0.1.0 (2026-04-09)


### Features

* add export command and OCI API layer ([7d237a6](https://github.com/devcontainer-env/devcontainer-env/commit/7d237a6d966b5adf566d9edd57b6cc15a4075970))
* add nix flake and devcontainer configuration ([b57a53c](https://github.com/devcontainer-env/devcontainer-env/commit/b57a53c1c1ea5578f8e63f737c2f42085217f0ba))
* **cli:** add clap-based CLI argument parser ([1f0634e](https://github.com/devcontainer-env/devcontainer-env/commit/1f0634e4f1b24be2831ecf8eaa000352fad7545e))
* **devcontainer-env:** add Display implementations for OCI API types ([be99971](https://github.com/devcontainer-env/devcontainer-env/commit/be99971a13e3a5eeed619ddb8bbc63ecefa1cb0f))
* **devcontainer-env:** implement exec command ([4d06a62](https://github.com/devcontainer-env/devcontainer-env/commit/4d06a62307c8fac8d1ee2799f70e93d47bfe21e5))
* **devcontainer-env:** implement inspect command ([3833837](https://github.com/devcontainer-env/devcontainer-env/commit/3833837b04682b186343a77f2846a328d03ec9f2))
* mark devcontainer and use it for environment extraction ([51619ff](https://github.com/devcontainer-env/devcontainer-env/commit/51619fffe260284a813256d2441cbc5b6c388abb))
* **oci:** add container hosts support and fix display formatting ([26520b1](https://github.com/devcontainer-env/devcontainer-env/commit/26520b19b5259ee10e079546ac859b0bdff739ed))
* **oci:** add url-based port rewriting for environment variables ([0cea2b7](https://github.com/devcontainer-env/devcontainer-env/commit/0cea2b7733b2c3daa73dc8e0e4cff8939eed9a88))
* **oci:** extract and filter container environment variables ([01c80e8](https://github.com/devcontainer-env/devcontainer-env/commit/01c80e88bd10ac43b84616eb2bc1969d36f63b9b))
* **oci:** extract docker compose service name as primary container host ([5f76aaa](https://github.com/devcontainer-env/devcontainer-env/commit/5f76aaaa5e0a01bf82e75c4fed70cf75f4f22d63))
* **oci:** query containers from docker socket ([88b3f20](https://github.com/devcontainer-env/devcontainer-env/commit/88b3f2062f0b3440287383b8d14dcfa082851c8a))
* **oci:** rewrite container urls with published host ports ([80b28b9](https://github.com/devcontainer-env/devcontainer-env/commit/80b28b9fb5b507bab0ad49d9e270a66ef00760b4))
* scaffold devcontainer-env CLI project ([e8d6b1c](https://github.com/devcontainer-env/devcontainer-env/commit/e8d6b1c2744c6f3b7c83b8e6e9df5da4ce2e8a1b))

## 0.1.0 (2026-04-09)


### Features

* add export command and OCI API layer ([7d237a6](https://github.com/devcontainer-env/devcontainer-env/commit/7d237a6d966b5adf566d9edd57b6cc15a4075970))
* add nix flake and devcontainer configuration ([b57a53c](https://github.com/devcontainer-env/devcontainer-env/commit/b57a53c1c1ea5578f8e63f737c2f42085217f0ba))
* **cli:** add clap-based CLI argument parser ([1f0634e](https://github.com/devcontainer-env/devcontainer-env/commit/1f0634e4f1b24be2831ecf8eaa000352fad7545e))
* **devcontainer-env:** add Display implementations for OCI API types ([be99971](https://github.com/devcontainer-env/devcontainer-env/commit/be99971a13e3a5eeed619ddb8bbc63ecefa1cb0f))
* **devcontainer-env:** implement exec command ([4d06a62](https://github.com/devcontainer-env/devcontainer-env/commit/4d06a62307c8fac8d1ee2799f70e93d47bfe21e5))
* **devcontainer-env:** implement inspect command ([3833837](https://github.com/devcontainer-env/devcontainer-env/commit/3833837b04682b186343a77f2846a328d03ec9f2))
* mark devcontainer and use it for environment extraction ([51619ff](https://github.com/devcontainer-env/devcontainer-env/commit/51619fffe260284a813256d2441cbc5b6c388abb))
* **oci:** add container hosts support and fix display formatting ([26520b1](https://github.com/devcontainer-env/devcontainer-env/commit/26520b19b5259ee10e079546ac859b0bdff739ed))
* **oci:** add url-based port rewriting for environment variables ([0cea2b7](https://github.com/devcontainer-env/devcontainer-env/commit/0cea2b7733b2c3daa73dc8e0e4cff8939eed9a88))
* **oci:** extract and filter container environment variables ([01c80e8](https://github.com/devcontainer-env/devcontainer-env/commit/01c80e88bd10ac43b84616eb2bc1969d36f63b9b))
* **oci:** extract docker compose service name as primary container host ([5f76aaa](https://github.com/devcontainer-env/devcontainer-env/commit/5f76aaaa5e0a01bf82e75c4fed70cf75f4f22d63))
* **oci:** query containers from docker socket ([88b3f20](https://github.com/devcontainer-env/devcontainer-env/commit/88b3f2062f0b3440287383b8d14dcfa082851c8a))
* **oci:** rewrite container urls with published host ports ([80b28b9](https://github.com/devcontainer-env/devcontainer-env/commit/80b28b9fb5b507bab0ad49d9e270a66ef00760b4))
* scaffold devcontainer-env CLI project ([e8d6b1c](https://github.com/devcontainer-env/devcontainer-env/commit/e8d6b1c2744c6f3b7c83b8e6e9df5da4ce2e8a1b))

## 0.1.0 (2026-04-09)


### Features

* add export command and OCI API layer ([7d237a6](https://github.com/devcontainer-env/devcontainer-env/commit/7d237a6d966b5adf566d9edd57b6cc15a4075970))
* add nix flake and devcontainer configuration ([b57a53c](https://github.com/devcontainer-env/devcontainer-env/commit/b57a53c1c1ea5578f8e63f737c2f42085217f0ba))
* **cli:** add clap-based CLI argument parser ([1f0634e](https://github.com/devcontainer-env/devcontainer-env/commit/1f0634e4f1b24be2831ecf8eaa000352fad7545e))
* **devcontainer-env:** add Display implementations for OCI API types ([be99971](https://github.com/devcontainer-env/devcontainer-env/commit/be99971a13e3a5eeed619ddb8bbc63ecefa1cb0f))
* **devcontainer-env:** implement exec command ([4d06a62](https://github.com/devcontainer-env/devcontainer-env/commit/4d06a62307c8fac8d1ee2799f70e93d47bfe21e5))
* **devcontainer-env:** implement inspect command ([3833837](https://github.com/devcontainer-env/devcontainer-env/commit/3833837b04682b186343a77f2846a328d03ec9f2))
* mark devcontainer and use it for environment extraction ([51619ff](https://github.com/devcontainer-env/devcontainer-env/commit/51619fffe260284a813256d2441cbc5b6c388abb))
* **oci:** add container hosts support and fix display formatting ([26520b1](https://github.com/devcontainer-env/devcontainer-env/commit/26520b19b5259ee10e079546ac859b0bdff739ed))
* **oci:** add url-based port rewriting for environment variables ([0cea2b7](https://github.com/devcontainer-env/devcontainer-env/commit/0cea2b7733b2c3daa73dc8e0e4cff8939eed9a88))
* **oci:** extract and filter container environment variables ([01c80e8](https://github.com/devcontainer-env/devcontainer-env/commit/01c80e88bd10ac43b84616eb2bc1969d36f63b9b))
* **oci:** extract docker compose service name as primary container host ([5f76aaa](https://github.com/devcontainer-env/devcontainer-env/commit/5f76aaaa5e0a01bf82e75c4fed70cf75f4f22d63))
* **oci:** query containers from docker socket ([88b3f20](https://github.com/devcontainer-env/devcontainer-env/commit/88b3f2062f0b3440287383b8d14dcfa082851c8a))
* **oci:** rewrite container urls with published host ports ([80b28b9](https://github.com/devcontainer-env/devcontainer-env/commit/80b28b9fb5b507bab0ad49d9e270a66ef00760b4))
* scaffold devcontainer-env CLI project ([e8d6b1c](https://github.com/devcontainer-env/devcontainer-env/commit/e8d6b1c2744c6f3b7c83b8e6e9df5da4ce2e8a1b))

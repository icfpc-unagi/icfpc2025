# ICFPC 2025 - Team Unagi

Team Unagi's comprehensive system for the International Conference on Functional Programming Contest (ICFPC) 2025. This repository contains a sophisticated Rust-based infrastructure for managing cloud resources, web-based visualization, database operations, and automated deployment workflows for competitive programming solutions.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Architecture](#architecture)
- [Usage](#usage)
- [Development](#development)
- [Docker Containers](#docker-containers)
- [Security](#security)
- [Troubleshooting](#troubleshooting)
- [Contributing](#contributing)

## Overview

This system provides:

- **Cloud Infrastructure Management**: Automated provisioning and management of Google Cloud Platform (GCP) virtual machines and storage
- **Web-Based Visualization**: Interactive frontend for analyzing problems and solutions using WebAssembly
- **Database Operations**: Centralized data storage and retrieval with MySQL backend
- **Automated Testing & Deployment**: CI/CD pipeline for building, testing, and deploying components
- **Secure Secret Management**: Encrypted credential handling for cloud operations

The system enables the team to rapidly deploy compute resources, visualize complex data, and manage the entire contest workflow through both web interfaces and command-line tools.

## Quick Start

1. **Set up environment**:
   ```bash
   export UNAGI_PASSWORD="your_password_here"
   ```

2. **Build the project**:
   ```bash
   cargo build
   ```

3. **Run tests**:
   ```bash
   make test
   ```

4. **List GCP instances**:
   ```bash
   ./run gcp instances --zone=asia-northeast1-b
   ```

5. **Start web server**:
   ```bash
   ./run www
   ```

## Prerequisites

- **Rust/Cargo**: Latest stable version
- **Docker**: For containerized deployment
- **Google Cloud Platform Access**: Valid GCP project with appropriate permissions
- **UNAGI_PASSWORD**: Environment variable for accessing encrypted secrets

### System Dependencies

```bash
# Ubuntu/Debian
sudo apt-get install clang build-essential pkg-config libssl-dev cmake

# For WebAssembly support
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

## Installation

1. **Clone the repository**:
   ```bash
   git clone https://github.com/icfpc-unagi/icfpc2025.git
   cd icfpc2025
   ```

2. **Set up environment variables**:
   ```bash
   export UNAGI_PASSWORD="your_password_here"
   ```

3. **Decrypt secrets** (if you have access):
   ```bash
   make secrets
   ```

4. **Build the project**:
   ```bash
   cargo build --release
   ```

5. **Run tests**:
   ```bash
   make test
   ```

## Architecture

### Core Components

#### 1. Database Layer (`src/sql.rs`)
- MySQL connection pooling with `CLIENT` static pool
- Query abstractions: `select()`, `row()`, `cell()`, `exec()`, `insert()`
- Custom `Row` wrapper for type-safe column access
- Foundation for all data persistence operations

#### 2. GCP Integration (`src/gcp/`)
- **Authentication** (`src/gcp/auth.rs`): OAuth2 service account management
- **Compute Engine** (`src/gcp/gce/`): VM instance creation and management
- **Cloud Storage** (`src/gcp/gcs/`): Object storage operations
- **CLI Tools** (`src/bin/gcp/`): Command-line interface with `instances`, `run`, `ls`, `cat` commands

#### 3. Web Application (`src/www/`)
- **Backend**: actix-web server with Handlebars templating
- **Frontend**: WebAssembly visualization components (`vis/`)
- **Handlers**: Request processing for API endpoints, tasks, leaderboard, etc.
- **Utilities**: Date/time formatting and web helpers

#### 4. WebAssembly Visualization (`vis/`)
- Rust-to-WASM compilation for browser-based visualization
- Interactive problem analysis and solution rendering
- Lightweight frontend components

#### 5. Build & Deploy Pipeline
- **Makefile**: Orchestrates testing, linting, Docker builds
- **Docker Containers**: Multi-stage builds for server, runner, builder, tools
- **CI/CD**: GitHub Actions for automated testing and linting
- **Secret Management**: Encrypted credential storage

### Directory Structure

```
├── src/                          # Main Rust source code
│   ├── bin/                      # Command-line executables
│   │   ├── gcp/                  # GCP management CLI
│   │   ├── hello.rs              # Simple test binary
│   │   ├── www.rs                # Web server binary
│   │   └── list_tables.rs        # Database schema inspector
│   ├── gcp/                      # Google Cloud Platform integration
│   │   ├── auth.rs               # OAuth2 authentication
│   │   ├── gce/                  # Compute Engine client
│   │   └── gcs/                  # Cloud Storage client
│   ├── www/                      # Web application components
│   │   └── handlers/             # HTTP request handlers
│   ├── lib.rs                    # Core library utilities
│   └── sql.rs                    # Database abstraction layer
├── vis/                          # WebAssembly visualization frontend
│   ├── src/lib.rs                # WASM entry points
│   ├── index.html                # Web interface
│   └── run.sh                    # Development server
├── docker/                       # Container definitions
│   ├── server.Dockerfile         # Main web server
│   ├── builder.Dockerfile        # Build environment
│   ├── runner.Dockerfile         # Execution environment
│   └── tools.Dockerfile          # Utility container
├── scripts/                      # Automation scripts
│   ├── deploy_binaries.sh        # Binary deployment to GCS
│   └── setup-instance.sh         # GCP instance setup
├── configs/                      # Encrypted configuration files
├── secrets/                      # Decrypted credentials (gitignored)
└── .github/workflows/            # CI/CD automation
```

## Usage

### GCP Command Line Interface

The GCP CLI provides unified access to Google Cloud Platform services:

#### List GCE Instances
```bash
./run gcp instances --zone=asia-northeast1-b --project=icfpc-primary
```

#### Create GCE Instance
```bash
./run gcp run --zone=asia-northeast1-b --machine-type=c2d-standard-4 my-vm 'echo hello'
```

#### List GCS Objects
```bash
# List bucket contents
./run gcp ls gs://icfpc2025/ -l

# Recursive listing
./run gcp ls gs://icfpc2025/ -R

# Object details
./run gcp ls gs://icfpc2025/specific-file.txt
```

#### Download GCS Object
```bash
./run gcp cat gs://icfpc2025/data/file.txt
```

### Web Application

Start the web server:
```bash
./run www
```

The web application provides:
- Task management and leaderboard
- Problem visualization
- API endpoints for contest data
- Administrative interfaces

### Database Operations

Inspect database schema:
```bash
./run list_tables
```

The system uses MySQL with connection pooling and provides type-safe query abstractions.

### WebAssembly Visualization

Build and run the visualization components:
```bash
cd vis
./run.sh
```

This compiles Rust code to WebAssembly for browser-based interactive visualizations.

## Development

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Build specific binary
cargo build --bin gcp
```

### Testing

```bash
# Run basic tests (excludes UNAGI-dependent tests)
make test

# Run all tests including those requiring UNAGI_PASSWORD
make test/unagi

# Run specific test
cargo test test_name
```

### Linting and Formatting

```bash
# Run linter (clippy with warnings as errors)
make lint

# Format code
make format

# Check formatting without changes
cargo fmt --check
```

### Adding New GCP Commands

1. Create new command file in `src/bin/gcp/commands/`:
   ```bash
   touch src/bin/gcp/commands/new_command.rs
   ```

2. Add module declaration in `src/bin/gcp/commands/mod.rs`:
   ```rust
   pub mod new_command;
   ```

3. Add command to enum in `src/bin/gcp/main.rs`:
   ```rust
   #[derive(Subcommand, Debug)]
   enum Commands {
       // ... existing commands
       NewCommand {
           // command arguments
       },
   }
   ```

4. Add match arm in main function:
   ```rust
   match cli.cmd {
       // ... existing matches
       Commands::NewCommand { /* args */ } => commands::new_command::run(/* args */).await,
   }
   ```

### Secret Management

Secrets are encrypted and stored in `configs/*.encrypted`. To work with secrets:

```bash
# Decrypt all secrets
make secrets

# Encrypt a new secret
echo "secret_content" | ./bin/encrypt > configs/new_secret.encrypted
```

**Important**: Never commit unencrypted secrets. Always use the encrypted versions.

## Docker Containers

The system provides several Docker containers for different purposes:

### Server Container (`docker/server.Dockerfile`)
- **Purpose**: Production web server with nginx, supervisord, and the main application
- **Build**: `make docker/server`
- **Usage**: Complete web application deployment
- **Ports**: 80 (HTTP)

### Builder Container (`docker/builder.Dockerfile`)
- **Purpose**: Rust compilation environment with all build dependencies
- **Build**: `make docker/builder`
- **Usage**: CI/CD builds, cross-compilation
- **Features**: Rust toolchain, WebAssembly support, native build tools

### Runner Container (`docker/runner.Dockerfile`)
- **Purpose**: Execution environment for contest problems
- **Build**: `make docker/runner`
- **Usage**: Running solutions in isolated environment
- **Features**: GCP integration, problem execution framework

### Tools Container (`docker/tools.Dockerfile`)
- **Purpose**: Development utilities and secret management
- **Build**: `make docker/tools`
- **Usage**: Encryption/decryption, development tasks
- **Tools**: openssl, make, jq, curl

### phpMyAdmin Container (`docker/phpmyadmin.Dockerfile`)
- **Purpose**: Web-based MySQL administration
- **Build**: `make docker/phpmyadmin`
- **Usage**: Database management and inspection
- **Authentication**: Uses UNAGI_PASSWORD

### Building and Running Containers

```bash
# Build all containers
make docker/server docker/builder docker/runner docker/tools

# Run web server container
docker run -p 80:80 -e UNAGI_PASSWORD="$UNAGI_PASSWORD" icfpc-unagi/server

# Use builder for compilation
docker run -v $(pwd):/work -w /work icfpc-unagi/builder cargo build --release

# Run tools for secret management
docker run -v $(pwd):/work -w /work icfpc-unagi/tools make secrets
```

## Security

### Environment Variables

- **UNAGI_PASSWORD**: Master password for decrypting secrets and authenticating services
  - Required for GCP authentication
  - Used for web application authentication
  - Never log or expose this value

### Secret Management

- All secrets are encrypted using `bin/encrypt` and stored in `configs/*.encrypted`
- Decrypted secrets are stored in `secrets/` (gitignored)
- GCP service account credentials are fetched from Cloud Storage using UNAGI_PASSWORD

### GCP Security

- Service account authentication via JWT tokens
- Scoped access to specific GCP services
- Instance creation requires appropriate IAM permissions
- All API calls use HTTPS with proper authentication

### Best Practices

1. **Never commit unencrypted secrets**
2. **Rotate UNAGI_PASSWORD regularly**
3. **Use least-privilege IAM roles**
4. **Monitor GCP resource usage and costs**
5. **Validate all user inputs in web handlers**

## Troubleshooting

### Common Issues

#### `UNAGI_PASSWORD not set`
```bash
export UNAGI_PASSWORD="your_password_here"
```

#### `Failed to download service_account.json`
- Check GCS permissions and network connectivity
- Verify UNAGI_PASSWORD is correct
- Ensure GCS bucket `icfpc2025-data` is accessible

#### `403 Forbidden` / `404 Not Found` (GCP API)
- Verify service account permissions and roles
- Check project ID and resource names
- Ensure target zone/region exists and is accessible

#### Clippy Errors
- Follow suggested fixes (e.g., `get(0)` → `first()`)
- Run `make format` to fix formatting issues
- Use `#[allow(clippy::lint_name)]` only as last resort

#### Build Failures
- Ensure all system dependencies are installed
- Check Rust version compatibility
- Clear target directory: `rm -rf target && cargo build`

#### Docker Issues
- Verify Docker daemon is running
- Check available disk space
- Ensure UNAGI_PASSWORD is set when building server container

### Database Issues

#### Connection Failures
- Verify MySQL server is running
- Check connection parameters in secrets
- Ensure database exists and user has permissions

#### Schema Issues
- Run `./run list_tables` to inspect current schema
- Check migration scripts in database setup

### Performance Issues

#### Slow GCP Operations
- Check network connectivity to GCP APIs
- Verify region/zone selection for optimal latency
- Monitor API rate limits and quotas

#### Web Application Performance
- Check database connection pool settings
- Monitor memory usage and garbage collection
- Verify static asset serving configuration

### Getting Help

1. **Check logs**: Application logs contain detailed error information
2. **Review AGENTS.md**: Comprehensive technical documentation in Japanese
3. **Inspect database**: Use phpMyAdmin container for database debugging
4. **Monitor resources**: Check GCP console for resource status and billing
5. **Test locally**: Use `make test` and `make test/unagi` for validation

## Contributing

### Development Workflow

1. **Create feature branch**:
   ```bash
   git checkout -b feature/description
   ```

2. **Make changes and test**:
   ```bash
   cargo build
   make test
   make lint
   ```

3. **Commit and push**:
   ```bash
   git add src/
   git commit -m "Add feature description"
   git push origin feature/description
   ```

4. **Create pull request** with detailed description

### Code Style

- Follow `cargo fmt` formatting
- Pass `clippy -D warnings` without errors
- Use meaningful variable and function names
- Add documentation for public APIs
- Write tests for new functionality

### Testing

- Add unit tests for new functions
- Include integration tests for GCP operations
- Test error handling and edge cases
- Verify Docker container builds

### Documentation

- Update README.md for user-facing changes
- Update AGENTS.md for technical details
- Add inline documentation for complex logic
- Include usage examples

---

## License

Copyright Team Unagi. See [LICENSE](LICENSE) for details.

## Links

- **Contest**: [ICFPC 2025](https://icfpcontest.org/)
- **Team**: Team Unagi
- **Repository**: [icfpc-unagi/icfpc2025](https://github.com/icfpc-unagi/icfpc2025)

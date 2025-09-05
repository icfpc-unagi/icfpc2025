# icfpc2025

Team Unagi's repository for ICFPC 2025 (International Conference on Functional Programming Contest).

This repository contains a Rust workspace with GCP tools, CLI utilities, and web components for the contest.

## Prerequisites

- Rust and Cargo (latest stable version)
- Docker (for some build operations)
- GCP access with appropriate permissions
- `UNAGI_PASSWORD` environment variable (required for GCP operations)

## Quick Start

### Environment Setup

Set the required environment variable:
```bash
export UNAGI_PASSWORD=your_password_here
```

### Build and Test

```bash
# Build all binaries
cargo build

# Run tests (excluding UNAGI-dependent tests)
make test

# Run tests that require UNAGI_PASSWORD
make test/unagi

# Lint code
make lint

# Format code
make format
```

### Running Applications

Use the provided launcher script for running binaries:

```bash
# GCP CLI examples
./run gcp ls gs://icfpc2025/ -l
./run gcp instances --zone=asia-northeast1-b
./run gcp run --zone=asia-northeast1-b --machine-type=c2d-standard-4 my-vm 'echo hello'

# Other binaries
./run www
./run hello
./run list_tables
```

## Project Structure

```
src/
  bin/
    gcp/                    # Main GCP CLI tool
    hello.rs               # Hello world example
    list_tables.rs         # Database table listing
    www.rs                 # Web server
  gcp/                     # GCP integration modules
    auth.rs               # GCP authentication
    gcs/                  # Google Cloud Storage
    gce/                  # Google Compute Engine
static/                   # Static assets
configs/                  # Configuration files
secrets/                  # Secret management
Makefile                  # Build and test targets
run                       # Binary launcher script
```

## GCP CLI Usage

The main CLI tool provides several commands:

- `gcp ls <gs://bucket/path>` - List GCS objects (use `-l` for detailed view, `-R` for recursive)
- `gcp instances [--zone=<zone>]` - List GCE instances
- `gcp run [options] <name> <command>` - Create and run GCE instance

## Development

### Adding New Commands

1. Create a new file in `src/bin/gcp/commands/`
2. Add the module to `src/bin/gcp/commands/mod.rs`
3. Update the command enum and match statement in `src/bin/gcp/main.rs`
4. Run `make lint` to verify

### Code Style

- Follow `clippy -D warnings` standards
- Use `cargo fmt` for formatting
- Run `make lint` before committing

## Environment Variables

- `UNAGI_PASSWORD` (required): Used to download service account credentials from GCS
- `CARGO_TARGET_DIR` (optional): Custom target directory for builds

## Documentation

For detailed Japanese documentation including architecture, security policies, and troubleshooting, see [AGENTS.md](AGENTS.md).

## License

See [LICENSE](LICENSE) file for details.

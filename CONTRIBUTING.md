# Contributing to Quantum Proxy

Thank you for your interest in contributing to Quantum Proxy! This document provides guidelines and instructions for contributing to this project.

## Code of Conduct

This project adheres to the [Contributor Covenant](https://www.contributor-covenant.org/) code of conduct. By participating, you are expected to uphold this code. Please report unacceptable behavior to the project maintainers.

## How to Contribute

### Reporting Issues

If you find a bug or have a feature request, please create an issue on GitHub. Before creating a new issue, please search existing issues to avoid duplicates.

When reporting an issue, please include:

- A clear and descriptive title
- A detailed description of the issue or feature request
- Steps to reproduce the problem (for bugs)
- Expected behavior and actual behavior
- Environment information (OS, Rust version, etc.)
- Any relevant logs or error messages

### Pull Requests

1. Fork the repository
2. Create a new branch for your changes (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Commit your changes (`git commit -m 'Add some amazing feature'`)
5. Push to the branch (`git push origin feature/amazing-feature`)
6. Submit a pull request

### Pull Request Guidelines

- Keep pull requests focused on a single change
- Update documentation and README.md as necessary
- Update examples to reflect your changes (if applicable)
- Ensure all tests pass before submitting
- Ensure your code follows the project's code style (using `cargo fmt` and `cargo clippy`)
- Add tests for your changes (if applicable)
- Keep the scope of your PR small, focused on a specific change

## Development Environment Setup

### Prerequisites

- Rust and Cargo (latest stable version)
- OpenSSL development libraries
- For hybrid certificate support: OQS OpenSSL fork and oqs-provider

### Setting Up Your Development Environment

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/quantum-proxy.git
   cd quantum-proxy
   ```

2. Install dependencies:
   ```bash
   # Install Rust tools
   rustup update
   rustup component add clippy rustfmt

   # Install OpenSSL development libraries (Ubuntu/Debian)
   sudo apt-get install libssl-dev

   # Or on macOS
   brew install openssl
   ```

3. Build the project:
   ```bash
   cargo build
   ```

4. Run tests:
   ```bash
   cargo test
   ```

### Working with Hybrid Certificates

For development with hybrid certificates, you'll need to install the OQS OpenSSL fork:

```bash
# Clone the OQS OpenSSL repository
git clone --branch OQS-OpenSSL_1_1_1-stable https://github.com/open-quantum-safe/openssl.git oqs-openssl
cd oqs-openssl

# Compile and install
./config --prefix=/opt/oqs-openssl shared
make -j$(nproc)
make install

# Set environment variables
export PATH="/opt/oqs-openssl/bin:$PATH"
export LD_LIBRARY_PATH="/opt/oqs-openssl/lib:$LD_LIBRARY_PATH"
```

## Code Style and Conventions

- Follow Rust's official style guidelines
- Use `cargo fmt` to format your code
- Use `cargo clippy` to check for common issues
- Write clear and concise comments in English
- Document public API with rustdoc comments
- Use meaningful variable and function names

## Testing

- Write unit tests for new functionality
- Ensure all tests pass before submitting a pull request
- Add integration tests for new features
- Test with both traditional and hybrid certificates when relevant

## Documentation

- Update documentation for any changes to the public API
- Use clear and concise language
- Include examples where appropriate
- Keep the README.md up to date

## Commit Messages

- Use clear and descriptive commit messages
- Use the present tense ("Add feature" not "Added feature")
- Keep the first line under 50 characters
- Add a blank line after the title, followed by a detailed description
- Reference issue numbers when applicable (e.g., "Fixes #123")

## Versioning

This project follows [Semantic Versioning](https://semver.org/). Please consider this when making changes.

## License

By contributing to this project, you agree that your contributions will be licensed under the project's [MIT License](LICENSE).

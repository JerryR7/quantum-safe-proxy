# Quantum Safe Proxy Documentation

This directory contains documentation for the Quantum Safe Proxy project.

## Available Documentation

| Document | Description |
|----------|-------------|
| [Comprehensive Guide](guide.md) | Complete guide covering installation, certificates, cryptography, utility scripts, and troubleshooting |

## Technology Stack

| Component | Technology |
|-----------|------------|
| **Language** | Rust |
| **TLS Library** | OpenSSL 3.5+ with built-in PQC support (also compatible with 3.6+, 3.7+) |
| **Proxy Runtime** | tokio + tokio-openssl |
| **Deployment** | Docker / Kubernetes / Systemd sidecar mode |
| **Certificate Tools** | OpenSSL 3.5+ CLI (hybrid CSR and certificates) |

## Supported Algorithms

| Type | Algorithms (OpenSSL 3.5+) | Description |
|------|---------------------------|-------------|
| **Key Exchange** | ML-KEM-512, ML-KEM-768, ML-KEM-1024 | NIST standardized post-quantum key encapsulation mechanisms (formerly Kyber) |
| **Signatures** | ML-DSA-44, ML-DSA-65, ML-DSA-87 | NIST standardized post-quantum digital signature algorithms (formerly Dilithium) |
| **Lattice-Based Signatures** | SLH-DSA-FALCON-512, SLH-DSA-FALCON-1024 | Stateless hash-based digital signature algorithms |
| **Hybrid Groups** | X25519MLKEM768, P256MLKEM768, P384MLKEM1024 | Hybrid key exchange combining classical and post-quantum algorithms |
| **Classical Fallback** | ECDSA (P-256, P-384, P-521), RSA, Ed25519 | Traditional algorithms for backward compatibility |

## Planned Documentation

The following documentation is planned for future development:

- **Installation Guide**: Detailed instructions for installing the Quantum Safe Proxy
- **Configuration Guide**: Complete reference for all configuration options
- **API Reference**: Documentation for programmatic interfaces
- **Troubleshooting Guide**: Common issues and solutions
- **Performance Tuning**: Optimizing the proxy for different environments

## External Resources

- [OpenSSL Documentation](https://www.openssl.org/docs/)
- [OpenSSL 3.5 Release Notes](https://www.openssl.org/news/openssl-3.5-notes.html)
- [OpenSSL 3.5 Post-Quantum Cryptography](https://www.openssl.org/docs/man3.5/man7/ossl-guide-pq.html)
- [Open Quantum Safe Project](https://openquantumsafe.org/)
- [NIST Post-Quantum Cryptography](https://csrc.nist.gov/projects/post-quantum-cryptography)
- [NIST PQC Standardization](https://csrc.nist.gov/Projects/post-quantum-cryptography/selected-algorithms-2022)

## Contributing to Documentation

Contributions to documentation are welcome! If you'd like to improve existing docs or add new ones, please follow these guidelines:

1. Use Markdown format for all documentation
2. Include a table of contents for longer documents
3. Provide code examples where appropriate
4. Include diagrams or images when they help explain concepts
5. Submit documentation changes via pull requests

For more information on contributing, see the [CONTRIBUTING.md](../CONTRIBUTING.md) file in the project root.

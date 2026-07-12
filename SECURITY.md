# Security Policy

## Supported Versions

| Version | Supported |
| ------- | --------- |
| latest  | Yes       |
| older   | No        |

Only the latest published release receives security updates.

## Reporting a Vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Report privately via [GitHub Security Advisories](https://github.com/Sigilweaver/OpenSXRaw/security/advisories/new).

Include:

- A description of the vulnerability and its potential impact.
- Steps to reproduce or a proof of concept (a small `.wiff`/`.wiff.scan`
  pair is ideal; small synthetic byte sequences are even better).
- The crate version and OS / toolchain.

Expect an initial acknowledgment within 7 days.

## Scope

In scope:

- **Parser correctness on malicious input.** OpenSXRaw parses a CFBF/OLE2
  container (`.wiff`) and a custom binary payload format (`.wiff.scan`).
  Panics, out-of-bounds reads, undefined behavior, infinite loops, or
  memory exhaustion triggered by a crafted or truncated file are in
  scope.
- **Memory safety**: the `opensxraw` crate forbids `unsafe_code`. A
  demonstrated unsafe-code violation reachable from safe API is a
  security bug.
- **Path-traversal or arbitrary-file-write bugs** in any helper that
  derives output paths from input filenames.
- **Supply-chain integrity** of published artifacts on crates.io.

Out of scope:

- Denial of service via legitimately large `.wiff.scan` files. Real
  acquisitions can be many GB by design.
- Inaccurate decoding of specific instrument acquisition modes. Those are
  correctness bugs - file them as regular issues.
- The `.wiff2` format being unreadable. That is a known, documented
  limitation (proprietary encryption, key not recovered), not a
  vulnerability - see
  [docs/format/03-wiff2-container.md](docs/format/03-wiff2-container.md).
- Vulnerabilities in third-party crates with no demonstrated exploit path
  through OpenSXRaw.

## Disclosure

We follow coordinated disclosure. Reporters are credited in the release
notes unless they prefer to remain anonymous. We aim to ship a fix within
30 days of confirming a high or critical issue.

## Note on reverse engineering

OpenSXRaw was developed by clean-room reverse engineering of public
artifacts (PRIDE deposits, the public CFBF/OLE2 container specification).
It does not depend on any SCIEX SDK or binary blob, and contains no SCIEX
proprietary code. Bug reports about parser accuracy or coverage are
welcome but are not security issues unless they involve one of the
categories above.

## Stack context

OpenSXRaw is one of several vendor readers in the
[OpenMassSpec](https://github.com/Sigilweaver/OpenMassSpec) stack. Sibling
readers: [OpenTFRaw](https://github.com/Sigilweaver/OpenTFRaw) (Thermo),
[OpenWRaw](https://github.com/Sigilweaver/OpenWRaw) (Waters),
[OpenTimsTDF](https://github.com/Sigilweaver/OpenTimsTDF) (Bruker),
[OpenARaw](https://github.com/Sigilweaver/OpenARaw) (Agilent). Shared
foundation: [openmassspec-core](https://github.com/Sigilweaver/OpenMassSpecCore).

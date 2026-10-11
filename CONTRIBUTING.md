# Contributing to OpenSXRaw

Thanks for your interest in OpenSXRaw. This is a small, single-maintainer
project that ships [Apache-2.0](LICENSE) Rust tooling for the open
mass-spec stack.

Crates in this repo: opensxraw.

## Contributing code (pull requests)

PRs are welcome for changes of any size, including large or breaking ones -
there's no requirement to open an issue first. That said, for larger changes
you may want to open an issue before writing code, especially if you're
unsure whether it fits the project's direction: a large PR that conflicts
with the roadmap can still be rejected even if the code itself is solid, and
an issue is a cheap way to check alignment before investing the time.

For any PR:

- Scope it to one logical change.
- Run `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings`
  locally. CI will run them too.
- Run `cargo test --workspace`.
- Update [CHANGELOG.md](CHANGELOG.md) under `## [Unreleased]` with a
  short bullet describing the user-visible change.
- Prefer [Conventional Commits](https://www.conventionalcommits.org/)
  (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`).
- Code is ASCII only and `#![forbid(unsafe_code)]`.

## Vendor software and clean-room policy

If you are contributing to the SCIEX `.wiff`/`.wiff.scan` reader, please
make sure new format knowledge came from public datasets and your own
analysis - **do not** copy or paste vendor SDK headers, sources,
decompiled code, or proprietary specifications. See
[ATTRIBUTION.md](ATTRIBUTION.md) and [CORPUS.md](CORPUS.md).

This section follows the Sigilweaver [format provenance
policy](https://github.com/Sigilweaver/ops/blob/main/PROVENANCE.md), shared
by every reader in the suite. Where the two differ, the policy wins.

**No vendor software in the project.** Do not depend on SCIEX's own tools,
or on anything that reads `.wiff`/`.wiff2` data through SCIEX's libraries,
and do not run them in CI or tests. That covers Analyst, SCIEX OS,
MultiQuant, and ProteoWizard `msconvert` (it links SCIEX's own `Clearcore2`
libraries to read this format, so it counts as vendor software here even
though it's open source). Do not bypass encryption, license checks, or any
other technical protection to read data, and do not contribute format
knowledge from anyone working from vendor-internal information.

Correctness is argued from open references: public data files, the
CFBF/OLE2 container format (a public Microsoft specification, not
SCIEX-specific), the PSI-MS mzML schema, published open specifications,
roundtrip and self-consistency invariants, and independent open-source
parsers used purely as format checkers. Every layout must be explainable
from those. If you can only explain a field by having watched what Analyst
shows for it, don't write that down - keep digging in the bytes instead, or
flag it as unresolved.

The `.wiff2` container is encrypted. Decrypting it is out of scope for this
project and contributions aimed at doing so will not be accepted (see
[docs/format/03-wiff2-container.md](docs/format/03-wiff2-container.md)).

**Cross-checking against vendor software is allowed, with disclosure.** If
you are licensed to run SCIEX software, you may compare values this project
decodes against its output. A comparison can confirm a decoder; it cannot
be the only basis for one. Say so in the pull request: which tool, which
files, and what you compared. Do not paste vendor-tool output into the
repository. If a value's only basis is a vendor comparison (for example a
derived quantity whose formula was matched to vendor output rather than
taken from a public source), document that next to the value. Cross-checks
are recorded in [ATTRIBUTION.md](ATTRIBUTION.md).

If you'd rather not write the fix yourself, or you've found a bug by
comparing against vendor software and don't want to send a pull request,
please open an issue instead. Describe the symptom on the input that
triggers it - what's wrong, and on what file - without pasting vendor tool
output or vendor source. We'll investigate and fix it from public
references. Detailed issue reports are genuinely useful and will be acted
on.

The local corpus checkout (the directory `OPENSXRAW_CORPUS` points at for
the conformance tests) holds real research data that can be expensive to redownload. Treat it as
read-only except for adding cache/derived files you clearly own.

## Security

See [SECURITY.md](SECURITY.md) for the vulnerability reporting process.

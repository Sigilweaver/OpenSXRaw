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

**Never use vendor software.** This is a clean-room project. Do not run,
depend on, or validate against SCIEX's own tools, or anything that reads
`.wiff`/`.wiff2` data through SCIEX's libraries - not in CI, not in tests,
not in local development. That means no Analyst, no SCIEX OS, no
MultiQuant, and no ProteoWizard `msconvert` (it links SCIEX's own
`Clearcore2` libraries to read this format, so it counts as vendor
software here even though it's open source). Do not install them, do not
shell out to them, do not use their output as a reference "to check your
work against."

Correctness is argued only from open references: the CFBF/OLE2 container
format (a public Microsoft specification, not SCIEX-specific), the PSI-MS
mzML schema, published open specifications, roundtrip and
self-consistency invariants, and independent open-source parsers used
purely as format checkers. Comparing, benchmarking, or tuning output
against vendor results is not allowed and would compromise the clean-room
status of the project. If you can only explain a field by having watched
what Analyst shows for it, don't write that down - keep digging in the
bytes instead, or flag it as unresolved.

The `.wiff2` container is proprietary-encrypted and currently unsolved
(see [docs/format/03-wiff2-container.md](docs/format/03-wiff2-container.md)).
Further structural/cryptanalytic reasoning from the ciphertext itself is
welcome, as long as it stays strictly clean-room public-domain analysis.
Do not disassemble or decompile any SCIEX binary, and do not send us key material obtained
from a leak, a breach, or any other unauthorized disclosure - we will not
accept or use it, regardless of how it's documented. If you believe you
have a legitimate external lead (e.g. something SCIEX has published
themselves), open an issue describing it before doing anything else so
we can assess it together.

**Pull requests that were written or verified with the help of proprietary
vendor software will not be accepted**, regardless of code quality, since
accepting them would compromise the project's clean-room provenance. If
you've found a bug this way, or you'd simply rather not write the fix
yourself, please open an issue instead. Describe the symptom on the input
that triggers it - what's wrong, and on what file - without pasting vendor
tool output, vendor source, or values you learned by running vendor
software. We'll investigate and fix it from public references. Detailed
issue reports are genuinely useful and will be acted on.

`/workspaces/Projects/Data` (or wherever the corpus lives in your checkout)
holds real research data that can be expensive to redownload. Treat it as
read-only except for adding cache/derived files you clearly own.

## Security

See [SECURITY.md](SECURITY.md) for the vulnerability reporting process.

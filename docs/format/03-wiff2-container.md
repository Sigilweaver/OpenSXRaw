# .wiff2 container format

Status: CONCLUDED, UNSOLVED (2026-07-08) - documented for anyone who picks this up later, not under active search

The `.wiff2` format combines the legacy `.wiff` metadata container and `.wiff.scan` data file into a single file. The internal structure is not publicly documented. This document records everything confirmed about the container's plaintext structure, every theory tried against its encryption, and what's still open.

## Sources - everything here is from public information

Every fact, hypothesis, and conclusion in this document traces back to one of the following public sources, and nothing else:

- **The corpus files themselves**: `.wiff`/`.wiff2` files downloaded from the EBI PRIDE Archive (https://www.ebi.ac.uk/pride/), a public proteomics data repository, published under CC-BY or equivalent open licenses by the labs that deposited them (full provenance record in `CORPUS.md`). Byte-level analysis of these files - entropy, structure, offsets - is analysis of our own lawfully-obtained data, not of any SCIEX-controlled system or software.
- **ProteoWizard (`pwiz`)**: a public, Apache-2.0-licensed open-source project whose own source code, build/packaging scripts, and git history were searched and read directly - e.g. to confirm which DLLs it redistributes from the vendor SDK, and to date `.wiff2` SDK support via its public commit history. `pwiz` itself links SCIEX's proprietary `Clearcore2` libraries to read `.wiff2` files; OpenSXRaw does not use `pwiz`, or anything that links those libraries, for anything - it was consulted purely as a public open-source reference, the same way any other public project would be.
- **Public code search**: GitHub searches for byte-sequence markers and terminology (e.g. the 8-byte plaintext-header anchor, "wiff2 SQLCipher", "PRAGMA key").
- **Public cryptographic references**: the SQLite file format specification, SQLCipher's own published design documentation, and `SQLite3MultipleCiphers` (an open-source project implementing several SQLite encryption schemes) - all public documentation, none SCIEX-specific.
- **SCIEX's own public-facing materials**: knowledge-base articles and marketing copy describing `.wiff2` as having "advanced data integrity mechanisms" - published, publicly accessible statements, not anything obtained under NDA, license, or other confidential arrangement.
- **Patent search**: public Google Patents search for relevant SCIEX filings.
- **Community sources**: public ProteoWizard/OpenMS issue trackers and public proteomics forum discussions.

See "Clean-room avenues exhausted" below for the specific results obtained from each of these.

**What was never used, at any point, by anyone working on this project**: the SCIEX SDK, any SCIEX software (Analyst, SCIEX OS, MultiQuant), any vendor binary or its disassembly/decompilation, any leaked or breach-sourced material, or any output from a dirty-room/contractor arrangement (none was ever engaged - see below).

**Why concluded:** the structural analysis below establishes a genuine plaintext/ciphertext boundary and rules out every well-known open-source SQLite encryption scheme as a direct match, but the encryption itself cannot be resolved from ciphertext alone with the information available. A dirty-room/disassembly approach (engaging a third party to examine the vendor binary, with a legal firewall keeping that work strictly separate from this clean-room project) was considered as a possible alternate path, but after reviewing SCIEX's EULA it was determined that route was not legally viable - it was never pursued: no contractor was ever engaged, no vendor binary was ever disassembled, and nothing from that route informed this document or this project in any way. Nathan's assessment (2026-07-08) is that further clean-room analysis isn't likely to resolve this without new external information. The theories below are preserved in full for whoever picks this up next - clean-room reverse engineering of legacy `.wiff`/`.wiff.scan` continues separately and is unaffected by this.

## Confirmed structural facts

Established by direct byte-level comparison across all 51 `.wiff2` files in the corpus (`Data/SRaw`), spanning 3 distinct PRIDE projects / institutions / instruments (PXD064013, PXD071083, PXD074536).

- Every file's size is an exact multiple of 4096 bytes (page-aligned, no partial trailing page).
- The file is organized as 4096-byte pages, consistent with SQLite's page structure.
- The first 32 bytes of page 0 (file offset 0-31) have a fixed structural pattern:

```
Bytes  0-15  (16 bytes): variable per file, 51/51 files distinct. High entropy.
                         Candidate: random salt, OR a file/session identifier
                         (see "GUID correlation" section below - unresolved).
Bytes 16-23  ( 8 bytes): IDENTICAL across all 51 corpus files:
                         10 00 01 01 0c 40 20 20
Bytes 24-31  ( 8 bytes): variable per file, 51/51 files distinct. High entropy.
                         Does NOT match real SQLite file-change-counter /
                         page-count values for the corresponding file
                         (checked directly - see "Plaintext header boundary"
                         below), so this region behaves like ciphertext,
                         not like the real SQLite fields that would
                         normally live at this offset.
Bytes 32+:               High-entropy payload, indistinguishable from random
                         at standard statistical tests (see Entropy section).
```

### The byte 16-23 constant decodes as genuine SQLite page-1 header fields

Interpreting `10 00 01 01 0c 40 20 20` using SQLite's own file-format spec (these fields normally live at header offset 16-23 in *any* SQLite database, encrypted or not):

| Sub-offset | Field | Value | Notes |
|---|---|---|---|
| 16-17 | Page size (big-endian u16) | `0x1000` = 4096 | Matches the independently-confirmed page alignment |
| 18 | File format write version | 1 | Legacy/non-WAL |
| 19 | File format read version | 1 | Legacy/non-WAL |
| 20 | Reserved bytes per page | 12 | See "The reserve=12 anomaly" below - this is the single most consequential number found so far |
| 21 | Max embedded payload fraction | 64 (`0x40`) | SQLite-spec-mandated constant, must always be 64 |
| 22 | Min embedded payload fraction | 32 (`0x20`) | SQLite-spec-mandated constant, must always be 32 |
| 23 | Leaf payload fraction | 32 (`0x20`) | SQLite-spec-mandated constant, must always be 32 |

Three of these eight bytes (21-23) are values the SQLite file format *requires* to always be 64/32/32 for any valid database, encrypted or not - landing on those exact values identically across 51 independent files from 3 unrelated institutions is not plausible as a ciphertext coincidence. This is read directly, not inferred or brute-forced.

### Why this proves a real plaintext/ciphertext boundary, not a lucky ciphertext match

A block cipher (AES-CBC, AES-ECB) cannot produce a ciphertext block that is byte-for-byte identical across files in its first 8 bytes while varying in its last 8 bytes - within a single fixed-size cipher block, the output depends on the *whole* block's plaintext input, so a clean 8-byte/8-byte split cannot arise from one 16-byte block being uniformly encrypted under a fixed key+IV. The only ways to produce exactly this pattern are: (a) a genuine plaintext/ciphertext boundary falling between byte 23 and byte 24, or (b) a byte-granular stream cipher (CTR, RC4, ChaCha-style) where per-byte XOR naturally allows a partial match within what would otherwise be one block's worth of output. Either way, **byte 24 is the most likely location where real encryption begins** - not byte 16 or byte 32, both of which were tried first and are now known wrong (see "Structural hypotheses tried" below).

This was corroborated, after the fact, by an unrelated source: `SQLite3MultipleCiphers` (an open-source project implementing 7 different SQLite encryption schemes) documents a real, named `plaintext_header_size` setting for exactly this purpose - exposing page-size/version/reserved/payload-fraction fields while encrypting the rest. Its documented behavior: values 1-23 are silently rounded up to 24, because that's structurally where the useful header fields end. That's the identical number independently derived here from raw corpus bytes, for the identical structural reason, before this library was known to exist.

## Entropy analysis

- Page 0 (bytes 0-4095): bytes 0-31 are the plaintext/structured region above; bytes 32-4095 are uniformly high entropy.
- Pages 1..N (bytes 4096+): the entirety of every subsequent page is uniformly high entropy.
- At 4KB window granularity: entropy ~7.95-7.96 bits/byte, chi-squared uniformity ~254-306 against a perfect uniform distribution - characteristic of strong cipher output (AES-class).
- At smaller window granularity (32/64-byte slices), entropy measures ~4.8-5.7 bits, which was initially misread as evidence of additional per-page plaintext sub-headers. This was a real methodological error, since corrected: calibrating the same measurement against `os.urandom()` shows *truly random* data measures the same ~4.8 bits at a 32-byte window (small-sample noise in the entropy estimator itself, not a structural signal). There are no per-page plaintext headers beyond page 0's first 24 bytes.
- No duplicate 4096-byte pages found anywhere in the scanned corpus (extended check: no shared byte-runs longer than the 8-byte anchor found anywhere in the first 64KB of any pair of files) - consistent with per-page IV/nonce variation, and separately confirms no accidental salt/key reuse across files (checked directly: two same-size files hashed and found to have fully distinct SHA-256, ruling out accidental duplication).

## Compression, ruled out

All of the following were tested directly against the binary content and ruled out before settling on the encryption hypothesis:
- Raw uncompressed floating-point payload (would produce entropy ~6.2-6.5, chi2 > 100,000)
- zlib with wrapper, zlib raw deflate (multiple candidate offsets)
- bz2, LZMA/XZ (multiple candidate offsets)
- LZ4, Snappy
- SQLite or HDF5 plaintext internal container
- Plaintext embedded ASCII/UTF-16 strings (0 found beyond statistical noise, past byte 31)

## Structural hypotheses tried against the encryption

In order attempted, each with what was learned:

### Hypothesis 1 (superseded): salt = bytes 0-15, ciphertext starts at byte 16

The "standard" reading of a SQLCipher-style file - a 16-byte salt immediately followed by ciphertext. This was the initial working assumption. **Now known wrong**: the plaintext-header analysis above shows bytes 16-23 are not ciphertext at all, so any construction built on this assumption checks the wrong bytes.

### Hypothesis 2 (superseded): salt = bytes 16-31, ciphertext starts at byte 32

The alternative tried alongside hypothesis 1 - treating the wiff2-specific leading 16 bytes as a non-cryptographic wrapper, with a second, "real" SQLCipher salt at offset 16-31 and ciphertext beginning at 32. **Also now known wrong**, for the same underlying reason: bytes 16-23 are constant across all files (inconsistent with being part of a random salt, which must be fully unpredictable), and the actual boundary is at 24, not 32.

### Hypothesis 3 (current best construction, validated but mode-dependent): ciphertext starts at byte 24

Once the offset-24 boundary was established, the block-chaining construction was re-derived from first principles and validated:

- First derivation attempt: a naive +8 shift of the earlier offsets (`iv=file[72:88]`, `enc=file[88:104]`). **Wrong** - failed to recover a self-created reference file's known password, catching the mistake before it was trusted.
- Second, correct construction: block grid re-derived from first principles, anchored at byte 24 (blocks at 24/40/56/72/88...). The SQLite "reserved for expansion, must be zero" region (spec offset 72-91) falls in block3 (file bytes 72-87), so that's the block that must be decrypted - using block2 (file bytes 56-71 ciphertext) as the CBC chaining "fake IV". Correct pairing: **iv=file[56:72], enc=file[72:88]**. Validated by successfully recovering a self-created reference file's passphrase (known passphrase, real plaintext header, genuine AES-256-CBC ciphertext with the zero-region actually zeroed) with this exact construction.

This hypothesis is validated as *internally consistent* - the construction correctly recovers a reference file's passphrase built with the same assumptions. It has **not** been validated against the real `.wiff2` target, because that requires already knowing the password. Its correctness against the real target still depends on two further assumptions that remain open (below).

## The reserve=12 anomaly, and the cipher-mode question

The reserve-bytes-per-page field (SQLite header offset 20, read directly per the plaintext-header analysis above) is **12** in every corpus file. This value does not fit any documented, well-known SQLite encryption scheme:

| Scheme | Reserve (bytes) | Source |
|---|---|---|
| SQLCipher, AES-CBC + HMAC-SHA1/256/512 | 16, 48, or 80 | Zetetic's own SQLCipher design docs. IV is randomly generated *and stored* per page (not derived); reserve is always rounded up to a multiple of the 16-byte AES block size - so 12 is arithmetically impossible under real SQLCipher, not just an unlikely configuration. |
| wxSQLite3, AES-CBC, no HMAC | 0 | IV is *derived*, not stored - clarifies that CBC alone doesn't force a nonzero reserve; it depends on whether the IV is random-and-stored vs. derived from key+page-number. |
| sqleet, ChaCha20-Poly1305 | 32 | 16-byte nonce + 16-byte Poly1305 tag. |
| Ascon-128 | 32 | 16-byte nonce + 16-byte tag. |
| AEGIS family | 48 or 64 | Depending on 16- or 32-byte nonce variant. |
| System.Data.SQLite, legacy RC4 | 0 | No HMAC needed for a stream cipher. Checked specifically because ProteoWizard's own public, open-source build/packaging scripts list "System.Data.SQLite" as a DLL they redistribute from the vendor SDK bundle - reserve size doesn't match, so this doesn't explain the observation, but the name match was worth ruling out explicitly. |

None of the seven schemes checked in `SQLite3MultipleCiphers` (a project that implements essentially the whole known open-source SQLite-encryption landscape) produce a reserve of exactly 12. **This is a real, comprehensive negative result**: `.wiff2` is very likely not a stock configuration of any well-known open-source SQLite encryption extension. This is consistent with SCIEX's own "advanced data integrity mechanisms" framing (marketing language for a custom/proprietary implementation) but doesn't hand us a ready-made answer either.

**Working alternative hypothesis, not yet confirmed or ruled out:** AES-GCM (or a similar AEAD construction) with a 12-byte nonce *derived* from salt+page-number (unstored, following the same derivation pattern as wxSQLite3's no-HMAC CBC scheme above) and a 12-byte *truncated* authentication tag stored in the page reserve. This is internally self-consistent with every byte observed, including the "no library matches" result - a bespoke truncated-tag AEAD scheme would naturally not match any off-the-shelf convention.

**Stronger evidence found while building a reference implementation to test the construction: reserve=12 combined with the confirmed offset-24 boundary is not just unusual, it's arithmetically incompatible with un-padded AES-CBC.** The ciphertext region (page bytes 24 through the start of the reserve) is `4096 - 24 - 12 = 4060` bytes - not a multiple of 16, AES's block size. CBC cannot encrypt/decrypt a non-block-aligned region without padding (which SQLite-style fixed-page-size encryption doesn't use - there's no room reserved for it, and the plaintext page content already exactly fills the usable space). This isn't dependent on which specific reserve value is "expected" for any particular library (the earlier table above); it's a direct consequence of the two independently-confirmed numbers (CT_START=24, reserve=12) not satisfying basic block arithmetic for CBC specifically. GCM/CTR-style stream ciphers have no equivalent constraint - they can encrypt/decrypt any byte length, which this finding is consistent with.

**A third hypothesis, added 2026-07-08 by Nathan, that resolves the block-alignment puzzle more cleanly than either alternative above: AES-CBC with ciphertext stealing (CBC-CTS).** CTS is a real, standard technique (used in things like disk-volume encryption and Kerberos) for encrypting a plaintext of *any* length under CBC without padding - it handles a non-block-aligned final chunk by "stealing" ciphertext bytes from the second-to-last block and reordering the last two blocks in the output, rather than padding the plaintext out to a block boundary. This directly resolves the 4,060-byte non-block-aligned ciphertext puzzle without needing to abandon CBC at all, and fits a bespoke/custom implementation at least as well as the GCM/AEAD hypothesis does (CTS is a known technique but rarely implemented, consistent with "unusual, not off-the-shelf").

**Important consistency check: this hypothesis does not change the offset-24 construction's validity.** CTS only changes behavior in the *last one or two blocks* of a CBC ciphertext region - everything before that decrypts under completely standard CBC chaining. The validated construction above operates on blocks 2-3 (file bytes 56-88), well within the bulk of the page and nowhere near the tail-end reserve boundary (~byte 4084) where CTS's special handling would actually kick in. If CBC-CTS is the real scheme, the offset-24 construction built and validated against a real reference file was structurally correct all along.

**A separate, clean piece of reasoning worth recording: the key is very likely a single static value embedded in the parser binary, not a per-install or server-issued key.** ProteoWizard (which links the vendor's `Clearcore2.Data.Wiff2.dll`) is reported on public forums to work fully air-gapped, with no network access required to open `.wiff2` files. That rules out any scheme where the key is fetched from a license server or cloud service at read time - the key (or everything needed to derive it) must be fully self-contained in the local SDK install. Combined with the earlier finding that no SCIEX product anywhere exposes a user-facing password for this, the picture that best fits all the evidence is one fixed key, compiled into the SDK, identical for every install and every customer.

**On the key's likely shape:** given the amount of engineering effort evident elsewhere in the format (custom plaintext-header handling, a scheme that matches no known library), a plausible guess is that the key itself is GUID-shaped rather than a human-chosen string - internal engineering teams commonly reach for a GUID when they need an opaque fixed constant, and a random 128-bit value would trivially explain why a password-style approach is unlikely to succeed at all. A real GUID-shaped brute force (all `8-4-4-4-12` hex combinations) is a 2^128 space, completely infeasible to exhaustively search; this only becomes tractable if a specific candidate GUID surfaces from somewhere (a leak, a public SDK artifact, a support forum post), not through search.

**This cannot be resolved from ciphertext alone**, and won't be, absent new external information. CBC (with or without CTS), CTR, and GCM all produce statistically uniform, indistinguishable-from-random ciphertext by design - that's the point of a well-implemented cipher mode, not a gap in this analysis. If the real mode is GCM/CTR rather than CBC, a CBC-chaining-based construction - even with the now-correct offset-24 boundary - would never work regardless of the correct key, because that technique specifically depends on CBC's `P_i = D_key(C_i) XOR C_{i-1}` relationship, which CTR/GCM don't have.

**Recommended next step (not yet done):** a fallback validation path that doesn't hard-depend on CBC block-chaining - a real PBKDF2+decrypt check (rather than a CBC-specific shortcut) that also tries a GCM/truncated-tag interpretation against a small candidate set, as a cheap sanity check on the mode question.

## GUID correlation - ruled out

The legacy `.wiff` container format (CFBF) has previously-undocumented streams under `MethodSubtree/Method1/` (`Wiff2BatchInfo`, `Wiff2MethodInfo`) and `SampleSubtree/Sample1/` (`Wiff2SampleInfo`) that reference a specific `.wiff2` conversion batch. `Wiff2BatchInfo` and `Wiff2SampleInfo` both end in the same 16-byte GUID. `Wiff2SampleInfo` additionally embeds a second, distinct GUID as UTF-8 text plus a long numeric suffix (purpose still undetermined); `Wiff2MethodInfo` contains a readable UTF-16LE acquisition method name.

**Tested hypothesis:** does either embedded GUID equal the paired `.wiff2` file's own leading 16 bytes (or the high-entropy bytes 24-31)? Initially inconclusive (the two `.wiff` files first checked, in PXD074536, don't have a same-stem `.wiff2` in the original corpus - a PRIDE upload gap). Resolved by finding genuinely paired same-stem `.wiff`+`.wiff2` runs directly via the PRIDE API - a broader scan across all 1,773 SCIEX-instrument projects on PRIDE (not just the 3 already in the corpus) found **62 same-stem pairs across 8 distinct projects** (PXD041697, PXD042133, PXD045599, PXD045877, PXD051873, PXD061811, PXD063182, PXD071194) - dual-format export turns out to be fairly common, not a rare edge case. Fetched one pair each from two different, unrelated projects (PXD071194 and PXD045599) and directly compared: **no match, in either project, between either embedded GUID and either candidate wiff2 byte region.** The known 8-byte constant anchor (`10 00 01 01 0c 40 20 20`) held in both, an incidental additional confirmation that it's universal across independent labs/instrument installs, not corpus-specific.

**Conclusion: the leading 16 bytes are not a discoverable batch/sample identifier reused from the paired `.wiff` file.** This doesn't rule out them being a genuine random cryptographic salt (the working default assumption), but it closes off what would have been the one shortcut around needing to actually recover the key.

## Clean-room avenues exhausted (no result)

- **Patent search**: Google Patents search for SCIEX filings mentioning "wiff2", "container format", "encryption", or "SQLCipher" - no implementation details found.
- **Code search**: GitHub searches for the 8-byte constant marker (`10 00 01 01 0c 40 20 20`) and combined queries ("wiff2 SQLCipher", "PRAGMA key") - zero relevant hits outside vendor DLL wrapper references.
- **Community/forums**: ProteoWizard and OpenMS issue trackers, proteomics forums - the community universally relies on the vendor's closed-source `Clearcore2.Data.Wiff2.dll`; no reverse-engineered keys or structural bypasses have ever been discussed or published.
- **Third-party open-source readers**: `MzIO.Wiff` (CSBiology, F#/NuGet) and `Wiff-Converter` (dmadea) both confirmed, via direct source inspection, to be thin wrappers around the vendor SDK requiring a valid Clearcore2 license file - neither reimplements any parsing or crypto, and neither touches `.wiff2` at all (legacy `.wiff` via Analyst only).
- **pwiz git history**: mining `ProteoWizard/pwiz`'s full commit history (182 branches, ~16.8k commits) for anything crypto-adjacent. "Disarm WIFF2 timebomb" commits turned out to be a Skyline test-suite date-based skip flag for flaky tests, unrelated to encryption/licensing. A content-level pickaxe search (`git log -S<term>` for `ClearCore2`, `SQLCipher`, `PBKDF`, etc.) found nothing crypto-relevant, mostly installer/DLL-packaging history (e.g. confirms wiff2 SDK support dates to SCIEX OS 1.2, consistent with the existing ~2014-2016 dating already used to justify targeting SQLCipher v3-era KDF parameters).
- **SCIEX's own public documentation**: KB articles confirm wiff2 encryption is mandatory and non-configurable, with no user-facing password/passphrase setting anywhere in the product line (Analyst, SCIEX OS, MultiQuant) - consistent with a fixed, developer-embedded key rather than a per-user passphrase, but doesn't reveal what that key or scheme is. The "SCIEX OS Security Database" (`Security.data`) is very likely operator/audit-trail login credentials (matches the 21 CFR Part 11 `CFR_INFO` stream already found in legacy `.wiff` files), not a wiff2 file-encryption key store - no evidence connecting the two.

## Open questions

1. **Cipher mode**: CBC (plain, or with ciphertext stealing) vs. GCM/AEAD with a derived nonce and truncated tag - not resolvable from ciphertext alone. CTS is the current best guess (2026-07-08), since it cleanly resolves the block-alignment puzzle without abandoning CBC, but is unconfirmed.
2. **KDF parameters**: iteration count and hash algorithm were never empirically confirmed. The working assumption (PBKDF2-HMAC-SHA1, 64,000 iterations, on historical SQLCipher-v3-era dating grounds) was always on shakier ground than it looked: it was inherited from "this looks like SQLCipher v3" reasoning, but the reserve=12 finding shows `.wiff2` almost certainly isn't a stock SQLCipher configuration at all - so a bespoke scheme could just as easily use a different iteration count, a different hash algorithm, or no per-password KDF whatsoever (plausible if the key is a raw GUID used directly, not passphrase-derived at all).
3. **Shared-key assumption**: whether all `.wiff2` files share one fixed key was argued circumstantially (no per-file user password exists anywhere in SCIEX's product line, and air-gapped operation rules out a server-issued key) but never directly confirmed.

Resolved, no longer open: whether bytes 0-15 are a discoverable identifier rather than genuine cryptographic entropy - ruled out above (GUID correlation section). They remain assumed to be a real random salt by elimination, not by direct proof.

## Conclusion

Unsolved, and no longer under active investigation as of 2026-07-08. Real, durable progress was made since the format was first marked "blocked": the plaintext-header boundary is understood and independently corroborated by an unrelated open-source project; the block-chaining construction anchored at byte 24 was corrected and validated against a known-good reference (twice, catching a real bug on the second pass); and a comprehensive cross-library comparison ruled out every well-known open-source SQLite encryption scheme as a direct match. What's left is real cryptographic uncertainty (CBC-CTS vs. GCM/AEAD, exact KDF parameters, and the key itself) that can't be resolved from ciphertext alone, absent new external information (a leaked key, a public SDK artifact). Legacy `.wiff`/`.wiff.scan` support is unaffected and covers the large majority of the corpus regardless of how this resolves.

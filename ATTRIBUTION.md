# Credits

## Prior art

There was no pre-existing open-source parser for SCIEX legacy `.wiff` /
`.wiff.scan` internals to draw on; the binary formats documented in
[docs/format/](docs/format/) were reverse-engineered from scratch against
the public PRIDE corpus (see [CORPUS.md](CORPUS.md)).

The `.wiff` container itself is Microsoft's public Compound File Binary
Format (CFBF/OLE2) - reading its stream tree is not reverse engineering
SCIEX's own work, only the contents and layout of the streams inside it
are. The `.wiff.scan` payload encoding (variable-length prefix tokens, the
per-block header, the `Idx` stream record layout) is SCIEX-specific and
was decoded entirely from corpus byte analysis.

`.wiff2`, SCIEX's newer self-contained format, is investigated and
documented (see
[docs/format/03-wiff2-container.md](docs/format/03-wiff2-container.md))
but not attributed to any external source - its proprietary encryption
scheme remains unsolved, and nothing in that document was learned from
SCIEX software, SDKs, or any third party.

## Standards

The mzML output follows the [HUPO-PSI mzML 1.1.0 specification](https://www.psidev.info/mzML)
and uses CV terms from the PSI-MS ontology (psi-ms.obo):

    Deutsch EW et al. "A guided tour of the Trans-Proteomic Pipeline."
    Proteomics. 2010;10(6):1150-9. doi:10.1002/pmic.200900375

## Validation corpus

Corpus files were downloaded from the [PRIDE Archive](https://www.ebi.ac.uk/pride/):

    Perez-Riverol Y et al. "The PRIDE database and related tools and resources in 2019:
    improving support for quantification data." Nucleic Acids Res. 2019;47(D1):D442-D450.
    doi:10.1093/nar/gky1106

## Rust dependencies

- [cfb](https://github.com/mdsteele/rust-cfb) -- Compound File Binary
  Format (CFBF/OLE2) reader (Matthew D. Steele, MIT)
- [byteorder](https://github.com/BurntSushi/byteorder) -- little/big-endian binary decoding (Andrew Gallant, MIT/Unlicense)
- [thiserror](https://github.com/dtolnay/thiserror) -- derive macro for Error impls (David Tolnay, MIT/Apache-2.0)

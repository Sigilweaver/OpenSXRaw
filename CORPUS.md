# OpenSXRaw Validation Corpus

Current size: 93.2 GB across 23 PRIDE projects, 250 runs. 199 of the 250
are complete legacy `.wiff` + `.wiff.scan` pairs (the format this reader
currently supports); the remaining 51 are `.wiff2` files, which are
investigated and documented but not yet readable - see
[docs/format/03-wiff2-container.md](docs/format/03-wiff2-container.md).

The corpus covers both TripleTOF (high-resolution, DDA/SWATH) and QTRAP
(nominal-mass, targeted MRM/SRM) instrument families, plus a small number
of ZenoTOF 7600 runs (SCIEX's newest platform, the source of most `.wiff2`
files in the corpus).

## Source: PRIDE Archive

All files come from the EBI PRIDE Archive (https://www.ebi.ac.uk/pride/),
a public proteomics repository:

    Perez-Riverol Y et al. "The PRIDE database and related tools and resources in 2019:
    improving support for quantification data." Nucleic Acids Res. 2019;47(D1):D442-D450.
    doi:10.1093/nar/gky1106

PRIDE datasets are published under CC-BY or equivalent open licences.

## Fetch tooling

`re/src/analysis/pride.py` (gitignored, local-only research tooling) is a
small CLI over the PRIDE REST API:

    python -m analysis.pride search <query>     # find SCIEX projects
    python -m analysis.pride files <accession>   # list a project's .wiff/.wiff2 files
    python -m analysis.pride fetch <accession>   # download .wiff/.wiff.scan pairs and .wiff2 files
    python -m analysis.pride catalog             # rebuild Data/SRaw/index.csv

## Provenance record

`Data/SRaw/index.csv` records which PRIDE project each local file came
from, plus its size, format generation, and pair completeness. To trace
any file back to its source, use the PRIDE accession:

    https://www.ebi.ac.uk/pride/archive/projects/<PXD_ACCESSION>

## Limitations

- The corpus is proteomics-focused (PRIDE's scope), so it skews toward
  DDA/SWATH/MRM proteomics acquisitions rather than every SCIEX
  application area (e.g. small-molecule/metabolomics workflows are
  under-represented).
- `.wiff2` support is blocked on the container's encryption scheme not
  yet being understood (see
  [docs/format/03-wiff2-container.md](docs/format/03-wiff2-container.md));
  the 51 `.wiff2` files in the corpus are retained for whenever that
  changes, not currently used for conformance testing.

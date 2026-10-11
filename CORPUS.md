# OpenSXRaw Validation Corpus

Current size: 93.4 GB across 25 PRIDE projects, 254 files:

- 201 legacy `.wiff` files (the format this reader supports). 198 have
  their `.wiff.scan` alongside; 3 do not. One of the 198 (in PXD075858)
  is a PRIDE submission text file named `.wiff`, not a `.wiff` container,
  so 197 pairs are readable.
- 53 `.wiff2` files, an encrypted container that this project does not
  support - see
  [docs/format/03-wiff2-container.md](docs/format/03-wiff2-container.md).

The 197 readable pairs are 170 TOF files (TripleTOF 6600, 5600+, 5600 and
two ZenoTOF 7600 `.wiff` files) and 27 QTRAP files (QTRAP 5500 and 6500+).

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
from, plus its size, format generation, and pair completeness. It
currently lists 23 of the 25 projects (250 files); PXD045599 and
PXD071194 are on disk but not yet in the index. Every file sits in a
directory named after its PRIDE accession, so any file can be traced back
to its source:

    https://www.ebi.ac.uk/pride/archive/projects/<PXD_ACCESSION>

## Limitations

- The corpus is proteomics-focused (PRIDE's scope), so it skews toward
  DDA/SWATH/MRM proteomics acquisitions rather than every SCIEX
  application area (e.g. small-molecule/metabolomics workflows are
  under-represented).
- `.wiff2` is an encrypted container and is out of scope (see
  [docs/format/03-wiff2-container.md](docs/format/03-wiff2-container.md));
  the 53 `.wiff2` files in the corpus are not used for conformance testing.

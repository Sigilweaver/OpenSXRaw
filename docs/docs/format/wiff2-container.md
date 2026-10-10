---
sidebar_position: 4
---

# .wiff2 container

`.wiff2` is SCIEX's newer single-file format, combining the metadata held in
a legacy `.wiff` file and the scan data held in its `.wiff.scan` companion.
It is an encrypted container.

Decrypting `.wiff2` is out of scope for this project. OpenSXRaw does not
read `.wiff2` files and there are no plans to add support. Use the legacy
`.wiff` + `.wiff.scan` pair when the acquisition software can export it.

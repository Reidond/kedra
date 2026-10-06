# Preserved research sources

Public repository records include this guide, citation metadata, hashes and
source ownership/findings. Raw PDFs, extracted bodies and fetched source captures
are retained locally and excluded from publication. A fresh clone can use the
canonical URLs in `citations.json`/the manifest to obtain documents under their
own terms, then compare the recorded hashes. Local PDF links in research notes
refer to optional captures; canonical source links remain the public references.

Archived on 2026-10-01 from the 18 PDFs supplied by the owner. Originals in
`papers/` are byte-for-byte copies; `SHA256SUMS` binds their bytes. `archive.json`
records stable IDs S-001 through S-018, original paths, byte counts, PDF page
counts, PDF metadata and SHA-256 hashes. PDF metadata is a discovery aid, not an
authoritative substitute for the title page and publication record.

Paths in `archive.json` are relative to the research workspace above this
directory; paths in `modern-archive.json` and the checksum manifests are relative
to this `sources/` directory. `citations.json` records these conventions.

`text/` contains searchable `pdftotext -layout` extracts. Extraction used Poppler
26.09.0; all 18 PDFs produced some text and successful extraction exit statuses.
One extractor emitted an xref-reconstruction diagnostic. Text is an aid: the
original PDF is authoritative for figures, mathematical notation and page-specific
claims. Form feeds separate PDF pages; citations in findings use one-based PDF
pages and printed pages/sections when useful. A successful extraction does not
establish that every article body is searchable.

`628612.pdf` is an entire 315-page VVSS2007 proceedings volume. Retaining the
supplied bytes preserves provenance; research analysis is limited to the Nix
build-farms contribution and its identifying publication information. The relevant
article occupies one-based PDF pages 76–88 (printed pages 65–77) and is scanned;
the initial text extraction misses its body. Owner O4 read all 13 rendered pages
and produced native macOS Vision OCR from 180-DPI Poppler renders. The derivative
`text/628612-nix-article-ocr.txt` and adjacent metadata retain that method, source
binding, page range and checksum. OCR can distort syntax; the original PDF remains
authoritative. It is not 315 pages of Nix-specific research.

`web/nixos-research.html` is the official research index snapshot. The adjacent
metadata records requested/resolved URL, access date and checksum. Modern official
documentation used in the synthesis is registered in `../source-manifest.md`.
`modern-archive.json` connects captured documents to exact URLs, access date,
observed versions/revisions and content hashes. S-023 is the additional 2025
author-hosted verified-interpreter paper, retained under `papers/` and `text/`.
Capture time is distinct from software version and publication date.

The Tvix packet uses its declared mirror pinned at
`9bed4ce6fc4c308f28df485b286a9aee467af686`. Raw README/architecture/selected code and
API revision/tree evidence are preserved as documents; none was executed. The
origin's access failure remains a provenance limit. The 2021 announcement's direct
HTML request failed; `web/tvix-rewriting-nix-rendered.json` and `.txt` retain a
separately labeled web-rendered capture. `modern-archive.json` keeps both the
failed raw request and successful fallback. Five later canonical PDF fetches
matched the owner's original hashes and alias those preserved files rather than
keeping duplicate payloads.

## Citation and preservation policy

Use stable source IDs in development documents together with a PDF page/section
and a link to the canonical publication or official documentation. The source
manifest connects IDs to originals; `bibliography.bib` supplies reusable entries.
Historical publications establish design ideas, not current implementation
behavior. Cite a dated/versioned modern source for claims about current Nix.

The scholarly originals retain their authors' and publishers' notices and terms;
the repository's software license does not relicense them. Derived summaries and
design proposals are separate files. This archive is local and uncommitted at
creation. Future public distribution must use the sources' actual terms.

To verify preserved originals from this directory with the existing host tool:

```sh
shasum -a 256 -c SHA256SUMS
shasum -a 256 -c modern-SHA256SUMS
```

This checks document preservation, not Nix semantics or product implementation.

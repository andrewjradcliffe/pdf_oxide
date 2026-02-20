# PDF Oxide v0.3.7 — Text Extraction Benchmark

**Date**: 2026-02-19
**Corpus**: 3,829 PDFs from 3 sources
**Baseline**: pymupdf 1.27.1
**Goal**: 100% clean (excluding broken PDFs)

## Tooling

### Step 1: Extract text with pdf_oxide (Rust)

```bash
cargo run --release --example bench_extract_all -- [--output /tmp/text_comparison]
```

- Source: `examples/bench_extract_all.rs`
- Output: `/tmp/text_comparison/pdf_oxide/results.csv` + `.txt` per PDF
- Walks 3 corpora, tries passwords, skips known bombs
- Typical runtime: ~10s for 3,829 PDFs

### Step 2: Extract text with pymupdf (Python)

```bash
python3 scripts/bench_pymupdf.py [--output /tmp/text_comparison]
```

- Source: `scripts/bench_pymupdf.py`
- Output: `/tmp/text_comparison/pymupdf/results.csv` + `.txt` per PDF
- Same corpora, same passwords, same skip list
- Typical runtime: ~14s for 3,829 PDFs

### Step 3: Compare results

```bash
python3 scripts/bench_compare.py [--output /tmp/text_comparison]
```

- Source: `scripts/bench_compare.py`
- Output: `/tmp/text_comparison/issues.csv` — every problem PDF with category, char counts, error, full path
- Classifies each PDF: clean, both_empty, oxide_less, oxide_empty, oxide_error, etc.
- Uses `pdf_path` as key (handles duplicate filenames across veraPDF subdirectories)

### Re-run after a fix

```bash
# Rebuild + re-extract oxide only (pymupdf results don't change)
cargo run --release --example bench_extract_all
# Re-compare
python3 scripts/bench_compare.py
```

## Corpus

| Source | PDFs | Description |
|--------|------|-------------|
| veraPDF-corpus | 2,907 | PDF/A compliance test suite |
| pdf.js (Mozilla) | 897 | Browser PDF rendering test suite |
| SafeDocs | 26 | Targeted edge cases |
| **Total** | **3,829** | (1 skipped: bomb_giant.pdf) |

## Baseline Results (2026-02-19)

| Status | Count | % |
|--------|------:|----:|
| **Clean** | **3,569** | **93.2%** |
| Issues | 260 | 6.8% |

### Clean breakdown

| Category | Count | % |
|----------|------:|----:|
| Equivalent text | 1,133 | 29.6% |
| Both empty (image/test PDFs) | 2,258 | 59.0% |
| Oxide has text, mupdf empty | 6 | 0.2% |
| Oxide more text (>1.2x) | 96 | 2.5% |
| Oxide much more text (>2x) | 66 | 1.7% |
| pymupdf error/crash | 10 | 0.3% |

### Issues breakdown

| Category | Count | % | Description |
|----------|------:|----:|-------------|
| Oxide less text (0.5–0.8x) | 117 | 3.1% | Partial extraction — spacing, encoding, missing text |
| Oxide empty, mupdf has text | 91 | 2.4% | Complete extraction failure |
| Oxide much less text (<0.5x) | 45 | 1.2% | Severe extraction gap |
| Oxide error/crash | 5 | 0.1% | Parse errors on malformed PDFs |
| Both error | 2 | 0.1% | Broken PDFs (both fail) |

## Root Cause Analysis: 253 Actionable Issues

(Excluding 7 error PDFs: 5 oxide_error + 2 both_error)

### oxide_empty — 91 PDFs where oxide gets nothing

| Root Cause | Count | Notes |
|-----------|------:|-------|
| Form field text (widgets) | 34 | pymupdf extracts widget values, oxide doesn't |
| Regular text (font/encoding) | 42 | Mostly 1–5 chars; some real text with CID/encoding failures |
| Annotation text | 12 | FreeText annotations, tx annotations |
| Brotli compression | 1 | Brotli-Prototype-FileA.pdf — 54,672 chars |

### oxide_much_less — 45 PDFs with <50% of pymupdf text

Top by gap:

| PDF | oxide | mupdf | ratio |
|-----|------:|------:|------:|
| TAMReview.pdf | 18,287 | 63,315 | 0.29 |
| issue6127.pdf | 6,613 | 13,397 | 0.49 |
| canvas.pdf | 1,180 | 4,255 | 0.28 |
| ThuluthFeatures.pdf | 811 | 2,060 | 0.39 |
| annotation-text-widget.pdf | 159 | 651 | 0.24 |

### oxide_less — 117 PDFs with 50–80% of pymupdf text

Top by char gap:

| PDF | oxide | mupdf | gap |
|-----|------:|------:|----:|
| issue13242.pdf | 2,051 | 2,969 | 918 |
| bug1019475_1.pdf | 1,200 | 1,645 | 445 |
| issue10640.pdf | 1,097 | 1,378 | 281 |
| issue17492.pdf | 304 | 581 | 277 |
| issue10900.pdf | 142 | 272 | 130 |

## Current Results (after annotation fix)

| Status | Count | % |
|--------|------:|----:|
| **Clean** | **3,674** | **96.0%** |
| Issues | 155 | 4.0% |

### Current issue breakdown

| Category | Count | % |
|----------|------:|----:|
| Oxide less text (0.5–0.8x) | 49 | 1.3% |
| Oxide much less text (<0.5x) | 41 | 1.1% |
| Oxide empty, mupdf has text | 58 | 1.5% |
| Oxide error/crash | 5 | 0.1% |
| Both error | 2 | 0.1% |

## Remaining Fix Priority

| # | Root Cause | PDFs | Impact | Effort |
|---|-----------|-----:|--------|--------|
| 1 | oxide_empty — font/encoding failures | ~40 | High — many small texts | Varied |
| 2 | oxide_less — encoding/ActualText/spacing | 49 | High — biggest bucket | Medium |
| 3 | oxide_much_less — big extraction gaps | 41 | High — real content missing | Hard |
| 4 | oxide_empty — Brotli decompression | 1 | Low (1 PDF) but 54K chars | Easy |
| 5 | oxide_error — parse failures | 5 | Low — malformed PDFs | Low priority |

## Progress Tracking

| Date | Clean | Clean % | Change | Notes |
|------|------:|--------:|-------:|-------|
| 2026-02-19 | 3,569 | 93.2% | — | Baseline with new tooling (raw char count comparison) |
| 2026-02-19 | 3,636 | 95.0% | +67 | Fixed compare: strip whitespace + control chars before comparing |
| 2026-02-19 | 3,674 | 96.0% | +38 | Annotation text extraction (Widget /V, FreeText, Stamp) + UTF-16BE/PDFDocEncoding decoding |
| 2026-02-19 | 3,683 | 96.2% | +9 | Fix symbolic font /Encoding ignored + fix Type0 OneByteIdentityH 2-byte decode bug |
| 2026-02-19 | 3,713 | 97.0% | +30 | Quote/DoubleQuote span fix, MAX_LINES removal, CID-as-Unicode fallback, XObject cycle detection, UTF-16LE, bare minus parser, Brotli support |
| 2026-02-19 | 3,725 | 97.3% | +12 | Improved compare: whitespace-normalized comparison for CID vertical font output |
| | | | | |

# PDFOxide

**Fast PDF Toolkit for Rust and Python**

Extract, create, and edit PDFs with Rust performance. Native Python bindings included.

[![Crates.io](https://img.shields.io/crates/v/pdf_oxide.svg)](https://crates.io/crates/pdf_oxide)
[![PyPI](https://img.shields.io/pypi/v/pdf_oxide.svg)](https://pypi.org/project/pdf_oxide/)
[![Documentation](https://docs.rs/pdf_oxide/badge.svg)](https://docs.rs/pdf_oxide)
[![Build Status](https://github.com/yfedoseev/pdf_oxide/workflows/CI/badge.svg)](https://github.com/yfedoseev/pdf_oxide/actions)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](https://opensource.org/licenses)

## Quick Start

### Python
```python
from pdf_oxide import PdfDocument

doc = PdfDocument("paper.pdf")
text = doc.extract_text(0)
chars = doc.extract_chars(0)
markdown = doc.to_markdown(0, detect_headings=True)
```

```bash
pip install pdf_oxide
```

### Rust
```rust
use pdf_oxide::PdfDocument;

let mut doc = PdfDocument::open("paper.pdf")?;
let text = doc.extract_text(0)?;
let images = doc.extract_images(0)?;
let markdown = doc.to_markdown(0, Default::default())?;
```

```toml
[dependencies]
pdf_oxide = "0.3"
```

## Why pdf_oxide?

- **Fast** — Rust core, mean 2.3ms per PDF, p99 = 20ms, 98.3% under 10ms
- **Reliable** — 100% pass rate on 3,830 test PDFs, zero panics, zero slow (>5s) PDFs
- **Complete** — Extract, create, and edit with one library
- **Dual-language** — First-class Rust API and Python bindings via PyO3

## Features

| Extract | Create | Edit |
|---------|--------|------|
| Text & Layout | Documents | Annotations |
| Images | Tables | Form Fields |
| Forms | Graphics | Bookmarks |
| Annotations | Templates | Links |
| Bookmarks | Images | Content |

## Python API

```python
from pdf_oxide import PdfDocument

doc = PdfDocument("report.pdf")
print(f"Pages: {doc.page_count}")
print(f"Version: {doc.version}")

# Extract text from each page
for i in range(doc.page_count):
    text = doc.extract_text(i)
    print(f"Page {i}: {len(text)} chars")

# Character-level extraction with positions
chars = doc.extract_chars(0)
for ch in chars:
    print(f"'{ch.char}' at ({ch.x:.1f}, {ch.y:.1f})")

# Password-protected PDFs
doc = PdfDocument("encrypted.pdf")
doc.authenticate("password")
text = doc.extract_text(0)
```

## Rust API

```rust
use pdf_oxide::PdfDocument;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = PdfDocument::open("paper.pdf")?;

    // Extract text
    let text = doc.extract_text(0)?;

    // Character-level extraction
    let chars = doc.extract_chars(0)?;

    // Extract images
    let images = doc.extract_images(0)?;

    // Vector graphics
    let paths = doc.extract_paths(0)?;

    Ok(())
}
```

## Performance

Verified against 3,830 PDFs from three independent test suites:

| Corpus | PDFs | Pass Rate |
|--------|-----:|----------:|
| veraPDF (PDF/A compliance) | 2,907 | 100% |
| Mozilla pdf.js | 897 | 99.2% |
| SafeDocs (targeted edge cases) | 26 | 100% |
| **Total** | **3,830** | **99.8%** |

| Metric | v0.3.5 | v0.3.6 |
|--------|--------|--------|
| **Mean latency** | 23.3ms | **2.3ms** (-90%) |
| **p50 latency** | 0.6ms | 0.7ms |
| **p90 latency** | 3.0ms | 3.0ms |
| **p99 latency** | 33ms | **20ms** (-39%) |
| **Max latency** | 68,722ms | **625ms** (-99%) |
| **Under 10ms** | 97.4% | **98.3%** |
| **Slow (>5s)** | 2 | **0** |
| **Timeouts** | 0 | 0 |
| **Panics** | 0 | 0 |

v0.3.6 eliminated two O(n) bottlenecks: page tree traversal (168× faster on 10,000-page PDFs) and xref miss scanning (146× faster on tagged PDFs). 100% pass rate on all valid PDFs — the 7 non-passing files across the corpus are intentionally broken test fixtures (missing PDF header, fuzz-corrupted catalogs, invalid xref streams).

## Installation

### Python

```bash
pip install pdf_oxide
```

Wheels available for Linux, macOS, and Windows. Python 3.8–3.14.

### Rust

```toml
[dependencies]
pdf_oxide = "0.3"
```

## Building from Source

```bash
# Clone and build
git clone https://github.com/yfedoseev/pdf_oxide
cd pdf_oxide
cargo build --release

# Run tests
cargo test

# Build Python bindings
maturin develop
```

## Documentation

- **[Getting Started (Rust)](docs/getting-started-rust.md)** - Complete Rust guide
- **[Getting Started (Python)](docs/getting-started-python.md)** - Complete Python guide
- **[API Docs](https://docs.rs/pdf_oxide)** - Full Rust API reference
- **[PDF Spec Reference](docs/spec/pdf.md)** - ISO 32000-1:2008

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

```bash
cargo build && cargo test && cargo fmt && cargo clippy -- -D warnings
```

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.

## Citation

```bibtex
@software{pdf_oxide,
  title = {PDF Oxide: Fast PDF Toolkit for Rust and Python},
  author = {Yury Fedoseev},
  year = {2025},
  url = {https://github.com/yfedoseev/pdf_oxide}
}
```

---

**Rust** + **Python** | 99.8% pass rate on 3,830 PDFs | mean 2.3ms | v0.3.6

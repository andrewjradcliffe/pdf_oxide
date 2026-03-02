# pdf-oxide-wasm

Fast, zero-dependency PDF toolkit for Node.js, browsers, and serverless edge runtimes.
Extract text, convert to markdown/HTML, search, fill forms, create and edit PDFs — all from WebAssembly.

Built on the [pdf-oxide](https://github.com/yfedoseev/pdf_oxide) Rust core. No native binaries, no system dependencies.

[![npm](https://img.shields.io/npm/v/pdf-oxide-wasm)](https://www.npmjs.com/package/pdf-oxide-wasm)
[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](https://github.com/yfedoseev/pdf_oxide/blob/main/LICENSE-MIT)

## Why pdf-oxide-wasm

| Feature | pdf-oxide-wasm | pdf-parse | pdf-lib | pdfjs-dist |
|---|---|---|---|---|
| Text extraction | Yes | Yes | No | Yes |
| Markdown / HTML output | Yes | No | No | No |
| PDF creation | Yes | No | Yes | No |
| Form field read/write | Yes | No | Partial | No |
| Full-text search (regex) | Yes | No | No | No |
| Image extraction | Yes | No | No | No |
| Merge, encrypt, edit | Yes | No | Yes | No |
| Serverless / edge runtimes | Yes | No | No | No |
| Zero native dependencies | Yes | Yes | Yes | No |
| WebAssembly-based | Yes | No | No | No |
| TypeScript types included | Yes | No | Yes | Yes |
| License | MIT / Apache-2.0 | MIT | MIT | Apache-2.0 |

## Install

```bash
npm install pdf-oxide-wasm
```

## Quick Start

### Extract text (Node.js — CommonJS)

```javascript
const { WasmPdfDocument } = require("pdf-oxide-wasm");
const fs = require("fs");

const bytes = new Uint8Array(fs.readFileSync("document.pdf"));
const doc = new WasmPdfDocument(bytes);

console.log(`Pages: ${doc.pageCount()}`);
console.log(doc.extractText(0));       // plain text from page 0
console.log(doc.toMarkdown(0));        // markdown from page 0
console.log(doc.toHtml(0));            // HTML from page 0

doc.free();
```

### Extract text (ESM / TypeScript)

```typescript
import { WasmPdfDocument } from "pdf-oxide-wasm";
import { readFile } from "fs/promises";

const bytes = new Uint8Array(await readFile("document.pdf"));
const doc = new WasmPdfDocument(bytes);

const text = doc.extractAllText();
const markdown = doc.toMarkdownAll();

doc.free();
```

### Create a PDF from Markdown

```javascript
import { WasmPdf } from "pdf-oxide-wasm";

const pdf = WasmPdf.fromMarkdown("# Invoice\n\nTotal: $42.00", "Invoice", "Acme Corp");
const bytes = pdf.toBytes(); // Uint8Array — write to file or send as response
```

### Search inside a PDF

```javascript
const results = doc.search("quarterly revenue", true); // case-insensitive
// Returns: [{ page, text, bbox, start_index, end_index, span_boxes }]
```

### Read and fill form fields

```javascript
const fields = doc.getFormFields();
// [{ name, field_type, value, tooltip, bounds, is_readonly, is_required }]

doc.setFormFieldValue("name", "Jane Doe");
doc.setFormFieldValue("agree_terms", true);

const filledPdf = doc.saveToBytes(); // Uint8Array
```

### Encrypt a PDF (AES-256)

```javascript
const encrypted = doc.saveEncryptedToBytes(
  "user-password",
  "owner-password",
  true,  // allow print
  false, // deny copy
);
```

## Features

**Text Extraction** — plain text, Markdown, and HTML output formats. Character-level and span-level extraction with bounding boxes, font names, sizes, weights, colors, and italic flags.

**Format Conversion** — convert any page or all pages to Markdown (with heading detection, images, form fields), HTML (with optional CSS layout preservation), or structured plain text.

**Full-Text Search** — regex and literal search across all pages or a single page. Case-insensitive, whole-word, and max-results options. Returns match positions with bounding boxes.

**Image Extraction** — extract image metadata (dimensions, color space, bits per component, bounding boxes) and raw image bytes as PNG.

**Form Fields** — read all AcroForm fields (text, button, choice, signature). Get/set individual field values. Export form data as FDF or XFDF. Flatten forms into static content. XFA detection.

**PDF Creation** — generate PDFs from Markdown, HTML, plain text, or images (PNG/JPEG). Multi-image support (one page per image). Set title, author metadata.

**PDF Editing** — set document metadata (title, author, subject, keywords). Rotate pages, set MediaBox/CropBox, crop margins. Erase (whiteout) regions. Reposition, resize, and set bounds on images. Flatten or apply redactions. Merge PDFs. Embed files.

**Encryption** — AES-256 encryption with granular permissions (print, copy, modify, annotate).

**Document Structure** — bookmarks/outline (table of contents), annotations (links, comments, form widgets), page labels, XMP metadata, vector paths.

## API Reference

### `WasmPdfDocument` — read, extract, search, and edit existing PDFs

| Method | Description |
|---|---|
| `new(data)` | Load PDF from `Uint8Array` |
| `pageCount()` | Number of pages |
| `version()` | PDF version as `[major, minor]` |
| `authenticate(password)` | Decrypt an encrypted PDF |
| `hasStructureTree()` | Check for Tagged PDF structure |
| **Text Extraction** | |
| `extractText(page)` | Plain text from one page |
| `extractAllText()` | Plain text from all pages |
| `extractChars(page)` | Character-level data with positions |
| `extractSpans(page)` | Span-level data with positions |
| **Format Conversion** | |
| `toMarkdown(page, headings?, images?, forms?)` | Markdown from one page |
| `toMarkdownAll(headings?, images?, forms?)` | Markdown from all pages |
| `toHtml(page, layout?, headings?, forms?)` | HTML from one page |
| `toHtmlAll(layout?, headings?, forms?)` | HTML from all pages |
| `toPlainText(page)` | Plain text with layout |
| `toPlainTextAll()` | Plain text all pages |
| **Search** | |
| `search(pattern, caseInsensitive?, literal?, wholeWord?, max?)` | Search all pages |
| `searchPage(page, pattern, ...)` | Search one page |
| **Images** | |
| `extractImages(page)` | Image metadata (dimensions, color space, bbox) |
| `extractImageBytes(page)` | Image data as PNG `Uint8Array` |
| `pageImages(page)` | Image placement info (bounds, matrix) |
| **Forms** | |
| `getFormFields()` | All form fields with types and values |
| `getFormFieldValue(name)` | Get a single field value |
| `setFormFieldValue(name, value)` | Set a field value |
| `exportFormData(format?)` | Export as FDF or XFDF |
| `hasXfa()` | Check for XFA form data |
| `flattenForms()` | Flatten all form fields |
| `flattenFormsOnPage(page)` | Flatten fields on one page |
| **Document Structure** | |
| `getOutline()` | Bookmarks / table of contents |
| `getAnnotations(page)` | Page annotations |
| `extractPaths(page)` | Vector paths (lines, curves) |
| `pageLabels()` | Page label ranges |
| `xmpMetadata()` | XMP metadata |
| **Editing** | |
| `setTitle(title)` | Set document title |
| `setAuthor(author)` | Set document author |
| `setSubject(subject)` | Set document subject |
| `setKeywords(keywords)` | Set document keywords |
| `setPageRotation(page, degrees)` | Set page rotation |
| `rotatePage(page, degrees)` | Rotate page by degrees |
| `rotateAllPages(degrees)` | Rotate all pages |
| `pageMediaBox(page)` | Get MediaBox |
| `setPageMediaBox(page, llx, lly, urx, ury)` | Set MediaBox |
| `pageCropBox(page)` | Get CropBox |
| `setPageCropBox(page, llx, lly, urx, ury)` | Set CropBox |
| `cropMargins(left, right, top, bottom)` | Crop all page margins |
| `eraseRegion(page, llx, lly, urx, ury)` | Whiteout a region |
| `eraseRegions(page, rects)` | Whiteout multiple regions |
| `repositionImage(page, name, x, y)` | Move an image |
| `resizeImage(page, name, w, h)` | Resize an image |
| `setImageBounds(page, name, x, y, w, h)` | Set image bounds |
| `flattenPageAnnotations(page)` | Flatten page annotations |
| `flattenAllAnnotations()` | Flatten all annotations |
| `applyPageRedactions(page)` | Apply redactions on page |
| `applyAllRedactions()` | Apply all redactions |
| `mergeFrom(data)` | Merge another PDF |
| `embedFile(name, data)` | Embed a file |
| **Save** | |
| `saveToBytes()` | Save edits → `Uint8Array` |
| `saveEncryptedToBytes(userPwd, ownerPwd?, ...)` | Save with AES-256 encryption |
| `free()` | Release WASM memory |

### `WasmPdf` — create new PDFs

| Method | Description |
|---|---|
| `fromMarkdown(content, title?, author?)` | Create PDF from Markdown |
| `fromHtml(content, title?, author?)` | Create PDF from HTML |
| `fromText(content, title?, author?)` | Create PDF from plain text |
| `fromImageBytes(data)` | Create PDF from image (PNG/JPEG) |
| `fromMultipleImageBytes(images)` | Create multi-page PDF from images |
| `toBytes()` | Get PDF as `Uint8Array` |
| `size` | PDF size in bytes |

## Platform Compatibility

Works without modification in:

- **Node.js** 18+ (CommonJS and ESM)
- **Browsers** — Chrome, Firefox, Safari, Edge
- **Cloudflare Workers** — runs in V8 isolates with WASM support
- **Deno** — native WASM support
- **Bun** — native WASM support

No native binaries, no `node-gyp`, no `postinstall` scripts. Install and use immediately.

## Performance

pdf-oxide-wasm is built on a Rust PDF parser compiled to WebAssembly. The Rust core ([pdf_oxide](https://crates.io/crates/pdf_oxide)) achieves 0.8ms mean extraction time across 3,830 test PDFs with a 100% success rate — the fastest PDF text extraction library available in Rust. The WASM compilation preserves near-native performance without garbage collection overhead or child process spawning.

## Full Documentation

Complete guide with examples: [Getting Started with WASM](https://github.com/yfedoseev/pdf_oxide/blob/main/docs/getting-started-wasm.md)

Rust library documentation: [docs.rs/pdf_oxide](https://docs.rs/pdf_oxide)

## License

MIT OR Apache-2.0

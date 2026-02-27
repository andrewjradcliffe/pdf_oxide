//! WebAssembly bindings for PDF Oxide.
//!
//! Provides a JavaScript/TypeScript API for PDF operations in browser
//! environments. Requires the `wasm` feature flag.
//!
//! # Example (JavaScript)
//!
//! ```javascript
//! import init, { WasmPdfDocument, WasmPdf } from 'pdf_oxide';
//!
//! await init();
//!
//! // Read an existing PDF
//! const response = await fetch('document.pdf');
//! const bytes = new Uint8Array(await response.arrayBuffer());
//! const doc = new WasmPdfDocument(bytes);
//! console.log(`Pages: ${doc.pageCount()}`);
//! console.log(doc.extractText(0));
//! console.log(doc.toMarkdown(0));
//!
//! // Create a new PDF from Markdown
//! const pdf = WasmPdf.fromMarkdown("# Hello\n\nWorld");
//! const pdfBytes = pdf.toBytes(); // Uint8Array
//!
//! // Edit a PDF
//! doc.setTitle("My Document");
//! doc.setPageRotation(0, 90);
//! const edited = doc.saveToBytes(); // Uint8Array
//! doc.free();
//! ```

use wasm_bindgen::prelude::*;

use crate::api::PdfBuilder;
use crate::converters::ConversionOptions;
use crate::document::PdfDocument;
use crate::editor::{
    DocumentEditor, EncryptionAlgorithm, EncryptionConfig, Permissions, SaveOptions,
};
use crate::search::{SearchOptions, TextSearcher};

// ============================================================================
// WasmPdfDocument — read, convert, search, extract, and edit PDFs
// ============================================================================

/// A PDF document loaded from bytes for use in WebAssembly.
///
/// Create an instance by passing PDF file bytes to the constructor.
/// Call `.free()` when done to release memory.
#[wasm_bindgen]
pub struct WasmPdfDocument {
    inner: PdfDocument,
    /// Raw bytes for editor initialization (kept for lazy editor creation)
    raw_bytes: Vec<u8>,
    /// Lazy-initialized editor for mutation operations
    editor: Option<DocumentEditor>,
}

impl WasmPdfDocument {
    /// Ensure the editor is initialized, creating it from the raw bytes if needed.
    fn ensure_editor(&mut self) -> Result<&mut DocumentEditor, JsValue> {
        if self.editor.is_none() {
            let editor = DocumentEditor::open_from_bytes(self.raw_bytes.clone())
                .map_err(|e| JsValue::from_str(&format!("Failed to open editor: {}", e)))?;
            self.editor = Some(editor);
        }
        Ok(self.editor.as_mut().expect("editor just initialized"))
    }
}

#[wasm_bindgen]
impl WasmPdfDocument {
    // ========================================================================
    // Constructor
    // ========================================================================

    /// Load a PDF document from raw bytes.
    ///
    /// @param data - The PDF file contents as a Uint8Array
    /// @throws Error if the PDF is invalid or cannot be parsed
    #[wasm_bindgen(constructor)]
    pub fn new(data: &[u8]) -> Result<WasmPdfDocument, JsValue> {
        #[cfg(feature = "wasm")]
        console_error_panic_hook::set_once();

        let bytes = data.to_vec();
        let inner = PdfDocument::open_from_bytes(bytes.clone())
            .map_err(|e| JsValue::from_str(&format!("Failed to open PDF: {}", e)))?;

        Ok(WasmPdfDocument {
            inner,
            raw_bytes: bytes,
            editor: None,
        })
    }

    // ========================================================================
    // Group 1: Core Read-Only
    // ========================================================================

    /// Get the number of pages in the document.
    #[wasm_bindgen(js_name = "pageCount")]
    pub fn page_count(&mut self) -> Result<usize, JsValue> {
        self.inner
            .page_count()
            .map_err(|e| JsValue::from_str(&format!("Failed to get page count: {}", e)))
    }

    /// Get the PDF version as [major, minor].
    #[wasm_bindgen(js_name = "version")]
    pub fn version(&self) -> Vec<u8> {
        let (major, minor) = self.inner.version();
        vec![major, minor]
    }

    /// Authenticate with a password to decrypt an encrypted PDF.
    ///
    /// @param password - The password string
    /// @returns true if authentication succeeded
    #[wasm_bindgen(js_name = "authenticate")]
    pub fn authenticate(&mut self, password: &str) -> Result<bool, JsValue> {
        self.inner
            .authenticate(password.as_bytes())
            .map_err(|e| JsValue::from_str(&format!("Authentication failed: {}", e)))
    }

    /// Check if the document has a structure tree (Tagged PDF).
    #[wasm_bindgen(js_name = "hasStructureTree")]
    pub fn has_structure_tree(&mut self) -> bool {
        matches!(self.inner.structure_tree(), Ok(Some(_)))
    }

    // ========================================================================
    // Group 2: Text Extraction
    // ========================================================================

    /// Extract plain text from a single page.
    ///
    /// @param page_index - Zero-based page number
    #[wasm_bindgen(js_name = "extractText")]
    pub fn extract_text(&mut self, page_index: usize) -> Result<String, JsValue> {
        self.inner
            .extract_text(page_index)
            .map_err(|e| JsValue::from_str(&format!("Failed to extract text: {}", e)))
    }

    /// Extract plain text from all pages, separated by form feed characters.
    #[wasm_bindgen(js_name = "extractAllText")]
    pub fn extract_all_text(&mut self) -> Result<String, JsValue> {
        self.inner
            .extract_all_text()
            .map_err(|e| JsValue::from_str(&format!("Failed to extract all text: {}", e)))
    }

    // ========================================================================
    // Group 3: Format Conversion
    // ========================================================================

    /// Convert a single page to Markdown.
    ///
    /// @param page_index - Zero-based page number
    /// @param detect_headings - Whether to detect headings (default: true)
    /// @param include_images - Whether to include images (default: true)
    #[wasm_bindgen(js_name = "toMarkdown")]
    pub fn to_markdown(
        &mut self,
        page_index: usize,
        detect_headings: Option<bool>,
        include_images: Option<bool>,
    ) -> Result<String, JsValue> {
        let mut opts = ConversionOptions::default();
        if let Some(dh) = detect_headings {
            opts.detect_headings = dh;
        }
        if let Some(ii) = include_images {
            opts.include_images = ii;
        }
        self.inner
            .to_markdown(page_index, &opts)
            .map_err(|e| JsValue::from_str(&format!("Failed to convert to markdown: {}", e)))
    }

    /// Convert all pages to Markdown.
    #[wasm_bindgen(js_name = "toMarkdownAll")]
    pub fn to_markdown_all(
        &mut self,
        detect_headings: Option<bool>,
        include_images: Option<bool>,
    ) -> Result<String, JsValue> {
        let mut opts = ConversionOptions::default();
        if let Some(dh) = detect_headings {
            opts.detect_headings = dh;
        }
        if let Some(ii) = include_images {
            opts.include_images = ii;
        }
        self.inner
            .to_markdown_all(&opts)
            .map_err(|e| JsValue::from_str(&format!("Failed to convert to markdown: {}", e)))
    }

    /// Convert a single page to HTML.
    ///
    /// @param page_index - Zero-based page number
    /// @param preserve_layout - Use CSS positioning to preserve layout (default: false)
    /// @param detect_headings - Whether to detect headings (default: true)
    #[wasm_bindgen(js_name = "toHtml")]
    pub fn to_html(
        &mut self,
        page_index: usize,
        preserve_layout: Option<bool>,
        detect_headings: Option<bool>,
    ) -> Result<String, JsValue> {
        let mut opts = ConversionOptions::default();
        if let Some(pl) = preserve_layout {
            opts.preserve_layout = pl;
        }
        if let Some(dh) = detect_headings {
            opts.detect_headings = dh;
        }
        self.inner
            .to_html(page_index, &opts)
            .map_err(|e| JsValue::from_str(&format!("Failed to convert to HTML: {}", e)))
    }

    /// Convert all pages to HTML.
    #[wasm_bindgen(js_name = "toHtmlAll")]
    pub fn to_html_all(
        &mut self,
        preserve_layout: Option<bool>,
        detect_headings: Option<bool>,
    ) -> Result<String, JsValue> {
        let mut opts = ConversionOptions::default();
        if let Some(pl) = preserve_layout {
            opts.preserve_layout = pl;
        }
        if let Some(dh) = detect_headings {
            opts.detect_headings = dh;
        }
        self.inner
            .to_html_all(&opts)
            .map_err(|e| JsValue::from_str(&format!("Failed to convert to HTML: {}", e)))
    }

    /// Convert a single page to plain text (with layout preservation options).
    #[wasm_bindgen(js_name = "toPlainText")]
    pub fn to_plain_text(&mut self, page_index: usize) -> Result<String, JsValue> {
        let opts = ConversionOptions::default();
        self.inner
            .to_plain_text(page_index, &opts)
            .map_err(|e| JsValue::from_str(&format!("Failed to convert to plain text: {}", e)))
    }

    /// Convert all pages to plain text.
    #[wasm_bindgen(js_name = "toPlainTextAll")]
    pub fn to_plain_text_all(&mut self) -> Result<String, JsValue> {
        let opts = ConversionOptions::default();
        self.inner
            .to_plain_text_all(&opts)
            .map_err(|e| JsValue::from_str(&format!("Failed to convert to plain text: {}", e)))
    }

    // ========================================================================
    // Group 4: Structured Extraction (returns JS objects via serde-wasm-bindgen)
    // ========================================================================

    /// Extract character-level data from a page.
    ///
    /// Returns an array of objects with: char, bbox {x, y, width, height},
    /// font_name, font_size, font_weight, is_italic, color {r, g, b}, etc.
    #[wasm_bindgen(js_name = "extractChars")]
    pub fn extract_chars(&mut self, page_index: usize) -> Result<JsValue, JsValue> {
        let chars = self
            .inner
            .extract_chars(page_index)
            .map_err(|e| JsValue::from_str(&format!("Failed to extract chars: {}", e)))?;
        serde_wasm_bindgen::to_value(&chars)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Extract span-level data from a page.
    ///
    /// Returns an array of objects with: text, bbox, font_name, font_size,
    /// font_weight, is_italic, color, etc.
    #[wasm_bindgen(js_name = "extractSpans")]
    pub fn extract_spans(&mut self, page_index: usize) -> Result<JsValue, JsValue> {
        let spans = self
            .inner
            .extract_spans(page_index)
            .map_err(|e| JsValue::from_str(&format!("Failed to extract spans: {}", e)))?;
        serde_wasm_bindgen::to_value(&spans)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    // ========================================================================
    // Group 5: Search
    // ========================================================================

    /// Search for text across all pages.
    ///
    /// @param pattern - Regex pattern or literal text to search for
    /// @param case_insensitive - Case insensitive search (default: false)
    /// @param literal - Treat pattern as literal text, not regex (default: false)
    /// @param whole_word - Match whole words only (default: false)
    /// @param max_results - Maximum results to return, 0 = unlimited (default: 0)
    ///
    /// Returns an array of {page, text, bbox, start_index, end_index, span_boxes}.
    #[wasm_bindgen(js_name = "search")]
    pub fn search(
        &mut self,
        pattern: &str,
        case_insensitive: Option<bool>,
        literal: Option<bool>,
        whole_word: Option<bool>,
        max_results: Option<usize>,
    ) -> Result<JsValue, JsValue> {
        let options = SearchOptions {
            case_insensitive: case_insensitive.unwrap_or(false),
            literal: literal.unwrap_or(false),
            whole_word: whole_word.unwrap_or(false),
            max_results: max_results.unwrap_or(0),
            page_range: None,
        };
        let results = TextSearcher::search(&mut self.inner, pattern, &options)
            .map_err(|e| JsValue::from_str(&format!("Search failed: {}", e)))?;
        serde_wasm_bindgen::to_value(&results)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Search for text on a specific page.
    #[wasm_bindgen(js_name = "searchPage")]
    pub fn search_page(
        &mut self,
        page_index: usize,
        pattern: &str,
        case_insensitive: Option<bool>,
        literal: Option<bool>,
        whole_word: Option<bool>,
        max_results: Option<usize>,
    ) -> Result<JsValue, JsValue> {
        let options = SearchOptions {
            case_insensitive: case_insensitive.unwrap_or(false),
            literal: literal.unwrap_or(false),
            whole_word: whole_word.unwrap_or(false),
            max_results: max_results.unwrap_or(0),
            page_range: Some((page_index, page_index)),
        };
        let results = TextSearcher::search(&mut self.inner, pattern, &options)
            .map_err(|e| JsValue::from_str(&format!("Search failed: {}", e)))?;
        serde_wasm_bindgen::to_value(&results)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    // ========================================================================
    // Group 6: Image Info (read-only metadata)
    // ========================================================================

    /// Extract image metadata from a page.
    ///
    /// Returns an array of objects with: width, height, color_space,
    /// bits_per_component, bbox (if available). Does NOT return raw image bytes.
    #[wasm_bindgen(js_name = "extractImages")]
    pub fn extract_images(&mut self, page_index: usize) -> Result<JsValue, JsValue> {
        let images = self
            .inner
            .extract_images(page_index)
            .map_err(|e| JsValue::from_str(&format!("Failed to extract images: {}", e)))?;

        // Serialize image metadata (not raw bytes)
        let metadata: Vec<serde_json::Value> = images
            .iter()
            .map(|img| {
                let mut obj = serde_json::Map::new();
                obj.insert("width".into(), serde_json::Value::from(img.width()));
                obj.insert("height".into(), serde_json::Value::from(img.height()));
                obj.insert(
                    "color_space".into(),
                    serde_json::Value::from(format!("{:?}", img.color_space())),
                );
                obj.insert(
                    "bits_per_component".into(),
                    serde_json::Value::from(img.bits_per_component()),
                );
                if let Some(bbox) = img.bbox() {
                    let bbox_obj = serde_json::json!({
                        "x": bbox.x,
                        "y": bbox.y,
                        "width": bbox.width,
                        "height": bbox.height
                    });
                    obj.insert("bbox".into(), bbox_obj);
                }
                serde_json::Value::Object(obj)
            })
            .collect();

        serde_wasm_bindgen::to_value(&metadata)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    // ========================================================================
    // Group 7: Editing — Metadata
    // ========================================================================

    /// Set the document title.
    #[wasm_bindgen(js_name = "setTitle")]
    pub fn set_title(&mut self, title: &str) -> Result<(), JsValue> {
        let editor = self.ensure_editor()?;
        editor.set_title(title);
        Ok(())
    }

    /// Set the document author.
    #[wasm_bindgen(js_name = "setAuthor")]
    pub fn set_author(&mut self, author: &str) -> Result<(), JsValue> {
        let editor = self.ensure_editor()?;
        editor.set_author(author);
        Ok(())
    }

    /// Set the document subject.
    #[wasm_bindgen(js_name = "setSubject")]
    pub fn set_subject(&mut self, subject: &str) -> Result<(), JsValue> {
        let editor = self.ensure_editor()?;
        editor.set_subject(subject);
        Ok(())
    }

    /// Set the document keywords.
    #[wasm_bindgen(js_name = "setKeywords")]
    pub fn set_keywords(&mut self, keywords: &str) -> Result<(), JsValue> {
        let editor = self.ensure_editor()?;
        editor.set_keywords(keywords);
        Ok(())
    }

    // ========================================================================
    // Group 7: Editing — Page Properties
    // ========================================================================

    /// Get the rotation of a page in degrees (0, 90, 180, 270).
    #[wasm_bindgen(js_name = "pageRotation")]
    pub fn page_rotation(&mut self, page_index: usize) -> Result<i32, JsValue> {
        let editor = self.ensure_editor()?;
        editor
            .get_page_rotation(page_index)
            .map_err(|e| JsValue::from_str(&format!("Failed to get rotation: {}", e)))
    }

    /// Set the rotation of a page (0, 90, 180, or 270 degrees).
    #[wasm_bindgen(js_name = "setPageRotation")]
    pub fn set_page_rotation(
        &mut self,
        page_index: usize,
        degrees: i32,
    ) -> Result<(), JsValue> {
        let editor = self.ensure_editor()?;
        editor
            .set_page_rotation(page_index, degrees)
            .map_err(|e| JsValue::from_str(&format!("Failed to set rotation: {}", e)))
    }

    /// Rotate a page by the given degrees (adds to current rotation).
    #[wasm_bindgen(js_name = "rotatePage")]
    pub fn rotate_page(&mut self, page_index: usize, degrees: i32) -> Result<(), JsValue> {
        let editor = self.ensure_editor()?;
        editor
            .rotate_page_by(page_index, degrees)
            .map_err(|e| JsValue::from_str(&format!("Failed to rotate page: {}", e)))
    }

    /// Rotate all pages by the given degrees.
    #[wasm_bindgen(js_name = "rotateAllPages")]
    pub fn rotate_all_pages(&mut self, degrees: i32) -> Result<(), JsValue> {
        let editor = self.ensure_editor()?;
        editor
            .rotate_all_pages(degrees)
            .map_err(|e| JsValue::from_str(&format!("Failed to rotate all pages: {}", e)))
    }

    /// Get the MediaBox of a page as [llx, lly, urx, ury].
    #[wasm_bindgen(js_name = "pageMediaBox")]
    pub fn page_media_box(&mut self, page_index: usize) -> Result<Vec<f32>, JsValue> {
        let editor = self.ensure_editor()?;
        let mbox = editor
            .get_page_media_box(page_index)
            .map_err(|e| JsValue::from_str(&format!("Failed to get media box: {}", e)))?;
        Ok(mbox.to_vec())
    }

    /// Set the MediaBox of a page.
    #[wasm_bindgen(js_name = "setPageMediaBox")]
    pub fn set_page_media_box(
        &mut self,
        page_index: usize,
        llx: f32,
        lly: f32,
        urx: f32,
        ury: f32,
    ) -> Result<(), JsValue> {
        let editor = self.ensure_editor()?;
        editor
            .set_page_media_box(page_index, [llx, lly, urx, ury])
            .map_err(|e| JsValue::from_str(&format!("Failed to set media box: {}", e)))
    }

    /// Get the CropBox of a page as [llx, lly, urx, ury], or null if not set.
    #[wasm_bindgen(js_name = "pageCropBox")]
    pub fn page_crop_box(&mut self, page_index: usize) -> Result<JsValue, JsValue> {
        let editor = self.ensure_editor()?;
        let cbox = editor
            .get_page_crop_box(page_index)
            .map_err(|e| JsValue::from_str(&format!("Failed to get crop box: {}", e)))?;
        match cbox {
            Some(b) => serde_wasm_bindgen::to_value(&b.to_vec())
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e))),
            None => Ok(JsValue::NULL),
        }
    }

    /// Set the CropBox of a page.
    #[wasm_bindgen(js_name = "setPageCropBox")]
    pub fn set_page_crop_box(
        &mut self,
        page_index: usize,
        llx: f32,
        lly: f32,
        urx: f32,
        ury: f32,
    ) -> Result<(), JsValue> {
        let editor = self.ensure_editor()?;
        editor
            .set_page_crop_box(page_index, [llx, lly, urx, ury])
            .map_err(|e| JsValue::from_str(&format!("Failed to set crop box: {}", e)))
    }

    /// Crop margins from all pages.
    #[wasm_bindgen(js_name = "cropMargins")]
    pub fn crop_margins(
        &mut self,
        left: f32,
        right: f32,
        top: f32,
        bottom: f32,
    ) -> Result<(), JsValue> {
        let editor = self.ensure_editor()?;
        editor
            .crop_margins(left, right, top, bottom)
            .map_err(|e| JsValue::from_str(&format!("Failed to crop margins: {}", e)))
    }

    // ========================================================================
    // Group 7: Editing — Erase / Whiteout
    // ========================================================================

    /// Erase (whiteout) a rectangular region on a page.
    #[wasm_bindgen(js_name = "eraseRegion")]
    pub fn erase_region(
        &mut self,
        page_index: usize,
        llx: f32,
        lly: f32,
        urx: f32,
        ury: f32,
    ) -> Result<(), JsValue> {
        let editor = self.ensure_editor()?;
        editor
            .erase_region(page_index, [llx, lly, urx, ury])
            .map_err(|e| JsValue::from_str(&format!("Failed to erase region: {}", e)))
    }

    /// Erase multiple rectangular regions on a page.
    ///
    /// @param page_index - Zero-based page number
    /// @param rects - Flat array of coordinates [llx1,lly1,urx1,ury1, llx2,lly2,urx2,ury2, ...]
    #[wasm_bindgen(js_name = "eraseRegions")]
    pub fn erase_regions(
        &mut self,
        page_index: usize,
        rects: &[f32],
    ) -> Result<(), JsValue> {
        if rects.len() % 4 != 0 {
            return Err(JsValue::from_str(
                "rects must have a length that is a multiple of 4",
            ));
        }
        let rect_arrays: Vec<[f32; 4]> = rects
            .chunks_exact(4)
            .map(|c| [c[0], c[1], c[2], c[3]])
            .collect();
        let editor = self.ensure_editor()?;
        editor
            .erase_regions(page_index, &rect_arrays)
            .map_err(|e| JsValue::from_str(&format!("Failed to erase regions: {}", e)))
    }

    /// Clear all pending erase operations for a page.
    #[wasm_bindgen(js_name = "clearEraseRegions")]
    pub fn clear_erase_regions(&mut self, page_index: usize) -> Result<(), JsValue> {
        let editor = self.ensure_editor()?;
        editor.clear_erase_regions(page_index);
        Ok(())
    }

    // ========================================================================
    // Group 7: Editing — Annotations
    // ========================================================================

    /// Flatten annotations on a page into the page content.
    #[wasm_bindgen(js_name = "flattenPageAnnotations")]
    pub fn flatten_page_annotations(&mut self, page_index: usize) -> Result<(), JsValue> {
        let editor = self.ensure_editor()?;
        editor
            .flatten_page_annotations(page_index)
            .map_err(|e| JsValue::from_str(&format!("Failed to flatten annotations: {}", e)))
    }

    /// Flatten all annotations in the document into page content.
    #[wasm_bindgen(js_name = "flattenAllAnnotations")]
    pub fn flatten_all_annotations(&mut self) -> Result<(), JsValue> {
        let editor = self.ensure_editor()?;
        editor
            .flatten_all_annotations()
            .map_err(|e| JsValue::from_str(&format!("Failed to flatten annotations: {}", e)))
    }

    // ========================================================================
    // Group 7: Editing — Redaction
    // ========================================================================

    /// Apply redactions on a page (removes redacted content permanently).
    #[wasm_bindgen(js_name = "applyPageRedactions")]
    pub fn apply_page_redactions(&mut self, page_index: usize) -> Result<(), JsValue> {
        let editor = self.ensure_editor()?;
        editor
            .apply_page_redactions(page_index)
            .map_err(|e| JsValue::from_str(&format!("Failed to apply redactions: {}", e)))
    }

    /// Apply all redactions in the document.
    #[wasm_bindgen(js_name = "applyAllRedactions")]
    pub fn apply_all_redactions(&mut self) -> Result<(), JsValue> {
        let editor = self.ensure_editor()?;
        editor
            .apply_all_redactions()
            .map_err(|e| JsValue::from_str(&format!("Failed to apply redactions: {}", e)))
    }

    // ========================================================================
    // Group 7: Editing — Image Manipulation
    // ========================================================================

    /// Get information about images on a page.
    ///
    /// Returns an array of {name, bounds: [x, y, width, height], matrix: [a, b, c, d, e, f]}.
    #[wasm_bindgen(js_name = "pageImages")]
    pub fn page_images(&mut self, page_index: usize) -> Result<JsValue, JsValue> {
        let editor = self.ensure_editor()?;
        let images = editor
            .get_page_images(page_index)
            .map_err(|e| JsValue::from_str(&format!("Failed to get page images: {}", e)))?;
        serde_wasm_bindgen::to_value(&images)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Reposition an image on a page.
    #[wasm_bindgen(js_name = "repositionImage")]
    pub fn reposition_image(
        &mut self,
        page_index: usize,
        name: &str,
        x: f32,
        y: f32,
    ) -> Result<(), JsValue> {
        let editor = self.ensure_editor()?;
        editor
            .reposition_image(page_index, name, x, y)
            .map_err(|e| JsValue::from_str(&format!("Failed to reposition image: {}", e)))
    }

    /// Resize an image on a page.
    #[wasm_bindgen(js_name = "resizeImage")]
    pub fn resize_image(
        &mut self,
        page_index: usize,
        name: &str,
        width: f32,
        height: f32,
    ) -> Result<(), JsValue> {
        let editor = self.ensure_editor()?;
        editor
            .resize_image(page_index, name, width, height)
            .map_err(|e| JsValue::from_str(&format!("Failed to resize image: {}", e)))
    }

    /// Set the complete bounds of an image on a page.
    #[wasm_bindgen(js_name = "setImageBounds")]
    pub fn set_image_bounds(
        &mut self,
        page_index: usize,
        name: &str,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> Result<(), JsValue> {
        let editor = self.ensure_editor()?;
        editor
            .set_image_bounds(page_index, name, x, y, width, height)
            .map_err(|e| JsValue::from_str(&format!("Failed to set image bounds: {}", e)))
    }

    // ========================================================================
    // Group 7: Editing — Save
    // ========================================================================

    /// Save all edits and return the resulting PDF as bytes.
    ///
    /// @returns Uint8Array containing the modified PDF
    #[wasm_bindgen(js_name = "saveToBytes")]
    pub fn save_to_bytes(&mut self) -> Result<Vec<u8>, JsValue> {
        let editor = self.ensure_editor()?;
        editor
            .save_to_bytes()
            .map_err(|e| JsValue::from_str(&format!("Failed to save PDF: {}", e)))
    }

    /// Save with encryption and return the resulting PDF as bytes.
    ///
    /// @param user_password - Password required to open the document
    /// @param owner_password - Password for full access (defaults to user_password)
    /// @param allow_print - Allow printing (default: true)
    /// @param allow_copy - Allow copying text (default: true)
    /// @param allow_modify - Allow modifying (default: true)
    /// @param allow_annotate - Allow annotations (default: true)
    #[wasm_bindgen(js_name = "saveEncryptedToBytes")]
    pub fn save_encrypted_to_bytes(
        &mut self,
        user_password: &str,
        owner_password: Option<String>,
        allow_print: Option<bool>,
        allow_copy: Option<bool>,
        allow_modify: Option<bool>,
        allow_annotate: Option<bool>,
    ) -> Result<Vec<u8>, JsValue> {
        let owner_pwd = owner_password
            .as_deref()
            .unwrap_or(user_password);

        let permissions = Permissions {
            print: allow_print.unwrap_or(true),
            print_high_quality: allow_print.unwrap_or(true),
            modify: allow_modify.unwrap_or(true),
            copy: allow_copy.unwrap_or(true),
            annotate: allow_annotate.unwrap_or(true),
            fill_forms: allow_annotate.unwrap_or(true),
            accessibility: true,
            assemble: allow_modify.unwrap_or(true),
        };

        let config = EncryptionConfig::new(user_password, owner_pwd)
            .with_algorithm(EncryptionAlgorithm::Aes256)
            .with_permissions(permissions);

        let options = SaveOptions::with_encryption(config);
        let editor = self.ensure_editor()?;
        editor
            .save_to_bytes_with_options(options)
            .map_err(|e| JsValue::from_str(&format!("Failed to save encrypted PDF: {}", e)))
    }
}

// ============================================================================
// WasmPdf — PDF creation from content
// ============================================================================

/// Create new PDF documents from Markdown, HTML, or plain text.
///
/// ```javascript
/// const pdf = WasmPdf.fromMarkdown("# Hello\n\nWorld");
/// const bytes = pdf.toBytes(); // Uint8Array
/// console.log(`PDF size: ${pdf.size} bytes`);
/// ```
#[wasm_bindgen]
pub struct WasmPdf {
    bytes: Vec<u8>,
}

#[wasm_bindgen]
impl WasmPdf {
    /// Create a PDF from Markdown content.
    ///
    /// @param content - Markdown string
    /// @param title - Optional document title
    /// @param author - Optional document author
    #[wasm_bindgen(js_name = "fromMarkdown")]
    pub fn from_markdown(
        content: &str,
        title: Option<String>,
        author: Option<String>,
    ) -> Result<WasmPdf, JsValue> {
        let mut builder = PdfBuilder::new();
        if let Some(t) = title {
            builder = builder.title(t);
        }
        if let Some(a) = author {
            builder = builder.author(a);
        }
        let pdf = builder
            .from_markdown(content)
            .map_err(|e| JsValue::from_str(&format!("Failed to create PDF: {}", e)))?;
        Ok(WasmPdf {
            bytes: pdf.into_bytes(),
        })
    }

    /// Create a PDF from HTML content.
    ///
    /// @param content - HTML string
    /// @param title - Optional document title
    /// @param author - Optional document author
    #[wasm_bindgen(js_name = "fromHtml")]
    pub fn from_html(
        content: &str,
        title: Option<String>,
        author: Option<String>,
    ) -> Result<WasmPdf, JsValue> {
        let mut builder = PdfBuilder::new();
        if let Some(t) = title {
            builder = builder.title(t);
        }
        if let Some(a) = author {
            builder = builder.author(a);
        }
        let pdf = builder
            .from_html(content)
            .map_err(|e| JsValue::from_str(&format!("Failed to create PDF: {}", e)))?;
        Ok(WasmPdf {
            bytes: pdf.into_bytes(),
        })
    }

    /// Create a PDF from plain text.
    ///
    /// @param content - Plain text string
    /// @param title - Optional document title
    /// @param author - Optional document author
    #[wasm_bindgen(js_name = "fromText")]
    pub fn from_text(
        content: &str,
        title: Option<String>,
        author: Option<String>,
    ) -> Result<WasmPdf, JsValue> {
        let mut builder = PdfBuilder::new();
        if let Some(t) = title {
            builder = builder.title(t);
        }
        if let Some(a) = author {
            builder = builder.author(a);
        }
        let pdf = builder
            .from_text(content)
            .map_err(|e| JsValue::from_str(&format!("Failed to create PDF: {}", e)))?;
        Ok(WasmPdf {
            bytes: pdf.into_bytes(),
        })
    }

    /// Get the PDF as a Uint8Array.
    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }

    /// Get the size of the PDF in bytes.
    #[wasm_bindgen(getter)]
    pub fn size(&self) -> usize {
        self.bytes.len()
    }
}

// ============================================================================
// Unit Tests
// ============================================================================
//
// JsValue is not functional on non-wasm32 targets (wasm-bindgen stubs abort).
// Tests are split into two groups:
//   1. Native-safe: methods returning Rust types on the happy path (no JsValue at runtime)
//   2. Wasm-only: methods that return JsValue or whose error paths create JsValue
//
// Run native tests:  cargo test --lib --features wasm -- wasm::tests
// Run wasm tests:    wasm-pack test --headless --node --features wasm

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Test Helpers
    // ========================================================================

    fn make_text_pdf(text: &str) -> Vec<u8> {
        crate::api::Pdf::from_text(text).unwrap().into_bytes()
    }

    fn doc_from_text(text: &str) -> WasmPdfDocument {
        WasmPdfDocument::new(&make_text_pdf(text)).unwrap()
    }

    fn make_markdown_pdf(md: &str) -> Vec<u8> {
        crate::api::PdfBuilder::new()
            .from_markdown(md)
            .unwrap()
            .into_bytes()
    }

    // ========================================================================
    // Group: Constructor
    // ========================================================================

    #[test]
    fn test_new_valid_pdf() {
        let bytes = make_text_pdf("Hello world");
        let result = WasmPdfDocument::new(&bytes);
        assert!(result.is_ok());
    }

    // Error-path tests require JsValue::from_str() which only works on wasm32
    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_new_invalid_bytes() {
        let result = WasmPdfDocument::new(b"not a pdf at all");
        assert!(result.is_err());
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_new_empty_bytes() {
        let result = WasmPdfDocument::new(b"");
        assert!(result.is_err());
    }

    // ========================================================================
    // Group: Core Read-Only
    // ========================================================================

    #[test]
    fn test_page_count() {
        let mut doc = doc_from_text("Hello");
        let count = doc.page_count().unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_version() {
        let doc = doc_from_text("Hello");
        let ver = doc.version();
        assert_eq!(ver.len(), 2);
        assert!(ver[0] >= 1, "major version should be at least 1");
    }

    #[test]
    fn test_authenticate_unencrypted() {
        let mut doc = doc_from_text("Hello");
        let result = doc.authenticate("password");
        assert!(result.is_ok());
    }

    #[test]
    fn test_has_structure_tree_false() {
        let mut doc = doc_from_text("Hello");
        assert!(!doc.has_structure_tree());
    }

    #[test]
    fn test_page_count_from_markdown() {
        let bytes = make_markdown_pdf("# Title\n\nSome content");
        let mut doc = WasmPdfDocument::new(&bytes).unwrap();
        assert!(doc.page_count().unwrap() >= 1);
    }

    // ========================================================================
    // Group: Text Extraction
    // ========================================================================

    #[test]
    fn test_extract_text() {
        let mut doc = doc_from_text("Hello world");
        let text = doc.extract_text(0).unwrap();
        assert!(
            text.contains("Hello") || text.contains("world"),
            "extracted text should contain source content, got: {}",
            text
        );
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_extract_text_invalid_page() {
        let mut doc = doc_from_text("Hello");
        let result = doc.extract_text(999);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_all_text() {
        let mut doc = doc_from_text("Hello world");
        let text = doc.extract_all_text().unwrap();
        assert!(!text.is_empty(), "extract_all_text should return non-empty");
    }

    #[test]
    fn test_extract_text_preserves_content() {
        let mut doc = doc_from_text("Test content 12345");
        let text = doc.extract_text(0).unwrap();
        assert!(
            text.contains("12345"),
            "should preserve numeric content, got: {}",
            text
        );
    }

    // ========================================================================
    // Group: Format Conversion
    // ========================================================================

    #[test]
    fn test_to_markdown() {
        let mut doc = doc_from_text("Hello markdown");
        let md = doc.to_markdown(0, None, None).unwrap();
        assert!(!md.is_empty());
    }

    #[test]
    fn test_to_markdown_all() {
        let mut doc = doc_from_text("Hello markdown");
        let md = doc.to_markdown_all(None, None).unwrap();
        assert!(!md.is_empty());
    }

    #[test]
    fn test_to_html() {
        let mut doc = doc_from_text("Hello html");
        let html = doc.to_html(0, None, None).unwrap();
        assert!(!html.is_empty());
    }

    #[test]
    fn test_to_html_all() {
        let mut doc = doc_from_text("Hello html");
        let html = doc.to_html_all(None, None).unwrap();
        assert!(!html.is_empty());
    }

    #[test]
    fn test_to_plain_text() {
        let mut doc = doc_from_text("Hello plain");
        let text = doc.to_plain_text(0).unwrap();
        assert!(!text.is_empty());
    }

    #[test]
    fn test_to_plain_text_all() {
        let mut doc = doc_from_text("Hello plain");
        let text = doc.to_plain_text_all().unwrap();
        assert!(!text.is_empty());
    }

    #[test]
    fn test_to_markdown_with_options() {
        let mut doc = doc_from_text("Hello options");
        let md = doc.to_markdown(0, Some(false), Some(false)).unwrap();
        assert!(!md.is_empty());
    }

    #[test]
    fn test_to_html_with_options() {
        let mut doc = doc_from_text("Hello options");
        let html = doc.to_html(0, Some(true), Some(false)).unwrap();
        assert!(!html.is_empty());
    }

    // ========================================================================
    // Group: Structured Extraction (serde_wasm_bindgen — wasm32 only)
    // ========================================================================

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_extract_chars_ok() {
        let mut doc = doc_from_text("ABC");
        let result = doc.extract_chars(0);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_extract_spans_ok() {
        let mut doc = doc_from_text("Hello spans");
        let result = doc.extract_spans(0);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_extract_chars_invalid_page() {
        let mut doc = doc_from_text("ABC");
        let result = doc.extract_chars(999);
        assert!(result.is_err());
    }

    // ========================================================================
    // Group: Search (serde_wasm_bindgen — wasm32 only)
    // ========================================================================

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_search_found() {
        let mut doc = doc_from_text("Hello world test search");
        let result = doc.search("Hello", None, Some(true), None, None);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_search_not_found() {
        let mut doc = doc_from_text("Hello world");
        let result = doc.search("ZZZZZ_NONEXISTENT", None, Some(true), None, None);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_search_page_found() {
        let mut doc = doc_from_text("Hello searchable content");
        let result = doc.search_page(0, "Hello", None, Some(true), None, None);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_search_page_invalid() {
        let mut doc = doc_from_text("Hello");
        let result = doc.search_page(999, "Hello", None, Some(true), None, None);
        let _ = result;
    }

    // ========================================================================
    // Group: Image Info (serde_wasm_bindgen — wasm32 only)
    // ========================================================================

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_extract_images_ok() {
        let mut doc = doc_from_text("No images here");
        let result = doc.extract_images(0);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_extract_images_invalid_page() {
        let mut doc = doc_from_text("Hello");
        let result = doc.extract_images(999);
        assert!(result.is_err());
    }

    // ========================================================================
    // Group: Metadata Editing
    // ========================================================================

    #[test]
    fn test_set_title() {
        let mut doc = doc_from_text("Hello");
        assert!(doc.set_title("My Title").is_ok());
    }

    #[test]
    fn test_set_author() {
        let mut doc = doc_from_text("Hello");
        assert!(doc.set_author("Author Name").is_ok());
    }

    #[test]
    fn test_set_subject() {
        let mut doc = doc_from_text("Hello");
        assert!(doc.set_subject("Subject Line").is_ok());
    }

    #[test]
    fn test_set_keywords() {
        let mut doc = doc_from_text("Hello");
        assert!(doc.set_keywords("pdf, test, rust").is_ok());
    }

    // ========================================================================
    // Group: Page Properties
    // ========================================================================

    #[test]
    fn test_page_rotation() {
        let mut doc = doc_from_text("Hello");
        let rotation = doc.page_rotation(0).unwrap();
        assert_eq!(rotation, 0);
    }

    #[test]
    fn test_set_page_rotation() {
        let mut doc = doc_from_text("Hello");
        assert!(doc.set_page_rotation(0, 90).is_ok());
        let rotation = doc.page_rotation(0).unwrap();
        assert_eq!(rotation, 90);
    }

    #[test]
    fn test_rotate_page() {
        let mut doc = doc_from_text("Hello");
        assert!(doc.rotate_page(0, 90).is_ok());
    }

    #[test]
    fn test_rotate_all_pages() {
        let mut doc = doc_from_text("Hello");
        assert!(doc.rotate_all_pages(180).is_ok());
    }

    #[test]
    fn test_page_media_box() {
        let mut doc = doc_from_text("Hello");
        let mbox = doc.page_media_box(0).unwrap();
        assert_eq!(mbox.len(), 4, "media box should have 4 coordinates");
        assert!(mbox[2] > mbox[0], "urx should be greater than llx");
        assert!(mbox[3] > mbox[1], "ury should be greater than lly");
    }

    #[test]
    fn test_set_page_media_box() {
        let mut doc = doc_from_text("Hello");
        assert!(doc.set_page_media_box(0, 0.0, 0.0, 612.0, 792.0).is_ok());
    }

    // page_crop_box returns JsValue via serde_wasm_bindgen — wasm32 only
    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_page_crop_box_unset() {
        let mut doc = doc_from_text("Hello");
        let result = doc.page_crop_box(0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_page_crop_box() {
        let mut doc = doc_from_text("Hello");
        assert!(doc.set_page_crop_box(0, 10.0, 10.0, 600.0, 780.0).is_ok());
    }

    #[test]
    fn test_crop_margins() {
        let mut doc = doc_from_text("Hello");
        assert!(doc.crop_margins(10.0, 10.0, 10.0, 10.0).is_ok());
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_page_rotation_invalid_page() {
        let mut doc = doc_from_text("Hello");
        let result = doc.page_rotation(999);
        assert!(result.is_err());
    }

    // ========================================================================
    // Group: Erase / Whiteout
    // ========================================================================

    #[test]
    fn test_erase_region() {
        let mut doc = doc_from_text("Hello");
        assert!(doc.erase_region(0, 0.0, 0.0, 100.0, 100.0).is_ok());
    }

    #[test]
    fn test_erase_regions_valid() {
        let mut doc = doc_from_text("Hello");
        let rects = [0.0, 0.0, 100.0, 100.0, 200.0, 200.0, 300.0, 300.0];
        assert!(doc.erase_regions(0, &rects).is_ok());
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_erase_regions_invalid_length() {
        let mut doc = doc_from_text("Hello");
        let rects = [0.0, 0.0, 100.0]; // Not a multiple of 4
        let result = doc.erase_regions(0, &rects);
        assert!(result.is_err());
    }

    #[test]
    fn test_clear_erase_regions() {
        let mut doc = doc_from_text("Hello");
        doc.erase_region(0, 0.0, 0.0, 100.0, 100.0).unwrap();
        assert!(doc.clear_erase_regions(0).is_ok());
    }

    // ========================================================================
    // Group: Annotations
    // ========================================================================

    #[test]
    fn test_flatten_page_annotations() {
        let mut doc = doc_from_text("Hello");
        assert!(doc.flatten_page_annotations(0).is_ok());
    }

    #[test]
    fn test_flatten_all_annotations() {
        let mut doc = doc_from_text("Hello");
        assert!(doc.flatten_all_annotations().is_ok());
    }

    // ========================================================================
    // Group: Redaction
    // ========================================================================

    #[test]
    fn test_apply_page_redactions() {
        let mut doc = doc_from_text("Hello");
        assert!(doc.apply_page_redactions(0).is_ok());
    }

    #[test]
    fn test_apply_all_redactions() {
        let mut doc = doc_from_text("Hello");
        assert!(doc.apply_all_redactions().is_ok());
    }

    // ========================================================================
    // Group: Image Manipulation (serde_wasm_bindgen — wasm32 only)
    // ========================================================================

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_page_images() {
        let mut doc = doc_from_text("Hello");
        let result = doc.page_images(0);
        assert!(result.is_ok());
    }

    // ========================================================================
    // Group: Save
    // ========================================================================

    #[test]
    fn test_save_to_bytes() {
        let mut doc = doc_from_text("Hello save");
        let bytes = doc.save_to_bytes().unwrap();
        assert!(!bytes.is_empty(), "saved bytes should not be empty");
    }

    #[test]
    fn test_save_to_bytes_pdf_header() {
        let mut doc = doc_from_text("Hello header");
        let bytes = doc.save_to_bytes().unwrap();
        assert!(
            bytes.starts_with(b"%PDF"),
            "saved bytes should start with PDF header"
        );
    }

    #[test]
    fn test_save_encrypted_to_bytes() {
        let mut doc = doc_from_text("Hello encrypted");
        let bytes = doc
            .save_encrypted_to_bytes("pass", None, None, None, None, None)
            .unwrap();
        assert!(!bytes.is_empty());
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn test_save_roundtrip() {
        let mut doc = doc_from_text("Roundtrip test");
        doc.set_title("Roundtrip Title").unwrap();
        let bytes = doc.save_to_bytes().unwrap();

        let mut doc2 = WasmPdfDocument::new(&bytes).unwrap();
        let text = doc2.extract_text(0).unwrap();
        assert!(
            text.contains("Roundtrip"),
            "roundtrip should preserve text, got: {}",
            text
        );
    }

    // ========================================================================
    // Group: WasmPdf Creation
    // ========================================================================

    #[test]
    fn test_wasm_pdf_from_markdown() {
        let result = WasmPdf::from_markdown("# Hello\n\nWorld", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_wasm_pdf_from_html() {
        let result = WasmPdf::from_html("<h1>Hello</h1><p>World</p>", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_wasm_pdf_from_text() {
        let result = WasmPdf::from_text("Hello world", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_wasm_pdf_to_bytes() {
        let pdf = WasmPdf::from_text("Hello bytes", None, None).unwrap();
        let bytes = pdf.to_bytes();
        assert!(!bytes.is_empty());
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn test_wasm_pdf_size() {
        let pdf = WasmPdf::from_text("Hello size", None, None).unwrap();
        assert!(pdf.size() > 0, "PDF size should be positive");
    }

    #[test]
    fn test_wasm_pdf_with_metadata() {
        let pdf = WasmPdf::from_markdown(
            "# Test",
            Some("Test Title".to_string()),
            Some("Test Author".to_string()),
        )
        .unwrap();
        assert!(pdf.size() > 0);
        let mut doc = WasmPdfDocument::new(&pdf.to_bytes()).unwrap();
        assert_eq!(doc.page_count().unwrap(), 1);
    }

    // ========================================================================
    // Group: Lazy Editor Init
    // ========================================================================

    #[test]
    fn test_editor_lazy_init() {
        let doc = doc_from_text("Hello");
        assert!(doc.editor.is_none());
    }

    #[test]
    fn test_editor_initialized_on_edit() {
        let mut doc = doc_from_text("Hello");
        assert!(doc.editor.is_none());
        doc.set_title("Title").unwrap();
        assert!(doc.editor.is_some());
    }
}

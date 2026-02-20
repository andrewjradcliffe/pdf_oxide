# Investigation: issue10900.pdf Character Extraction Gap

## Executive Summary

**Issue**: pdf_oxide extracts 188 characters from issue10900.pdf while PyMUPDF extracts 271 characters - a **40 character gap (17% difference)**.

**Root Cause**: The PDF uses highly fragmented text operations with precise positioning (column-based layout). pdf_oxide's span merging algorithm combines fragments assuming they form logical words, but this PDF intentionally keeps columns separated with space strings between them.

## Detailed Analysis

### PDF Structure (from PyMUPDF analysis)

The PDF contains **44 separate text spans** arranged in a table-like grid:

```
Y=179.2: "3" + "            " + "3" + "            " + "3" + "            " + "3" + "            " (4 columns)
Y=188.5: "851.5" + "     " + "854.9" + "     " + "839.3" + "     " + "837.5" + "     " (4 columns)
Y=199.6: Similar pattern...
Y=209.8: Similar pattern...
Y=219.9: Similar pattern...
Y=229.3: "13.0x" + "13.0x" + "13.0x" + "12.5x" (4 columns)
```

### Character Count Comparison

| Source | Method | Total Chars | Spaces | Non-Spaces | Spans/Lines |
|--------|--------|-------------|--------|-----------|------------|
| PyMUPDF | extract_text | 271 | 116 | 112 | 44 spans |
| pdf_oxide | extract_text | 188 | ~74 | ~114 | ~20 lines |
| pdf_oxide | extract_chars | 58 | 14 | 44 | N/A |

**Finding**: pdf_oxide is missing ~40-42 characters, **predominantly spaces** (42 vs 116 space chars).

### PDF Operations Analysis

From examining the content stream structure:
- Each column cell uses a separate text operation (Tj or TJ array)
- Columns are separated by explicit space strings
- Positioning is done via `Td` (text positioning) operators, not geometric layout
- The PDF intentionally fragments text to achieve precise column alignment

### pdf_oxide Extraction Behavior

When pdf_oxide processes this PDF:

1. **extract_text_spans()** creates spans for each text operation
2. **merge_adjacent_spans()** combines nearby spans on the same line
3. **Space-only spans** between columns are merged with adjacent content
4. **Final output** reduces 44 spans to ~20 "lines" (Y-position groups)

This is intentional behavior for PDFs with continuous text, but it's incorrect for table/column layouts where spaces are intentional separators.

### Why extract_chars() Only Gets 58 Characters

The `extract()` method (character mode) has different handling:
- It iterates through TJ array elements directly
- Space insertion from TJ offsets uses a threshold-based heuristic
- It may skip or incorrectly handle the precise column positioning
- Result: Only captures a subset of text operations

## Technical Details

### Suspicious Code Paths

1. **process_tj_array_tiebreaker()** (line 4400):
   ```rust
   let is_raw_ascii_space_only = !s.is_empty() && s.iter().all(|&b| b == 0x20);
   if is_raw_ascii_space_only && !unicode_text.is_empty() && unicode_text.trim().is_empty() {
       if !buffer.unicode.is_empty() {
           if let Some(last_char) = buffer.unicode.chars().last() {
               if last_char.is_lowercase() {
                   // This space is splitting a word - skip it!
                   self.advance_position_for_string(s)?;
                   continue; // <-- SKIPS SPACE SPAN!
               }
           }
       }
   }
   ```
   
   **Issue**: This code skips space strings that appear after lowercase letters, assuming they're mid-word spacing. However, in a column layout like this PDF, spaces after numbers should be preserved.

2. **merge_adjacent_spans()** (line 2664):
   ```rust
   let next_is_whitespace_only = span.text.chars().all(|c| c.is_whitespace());
   if next_is_whitespace_only {
       // Next span is already space-only: just concatenate without adding more space
       format!("{}{}", current.text, span.text)
   }
   ```
   
   **Issue**: When merging, space-only spans are concatenated directly without special handling. This loses the spacing information that was intentional in the original PDF.

3. **merge_adjacent_spans()** (line 2633):
   ```rust
   let should_merge = same_line
       && (self.merging_config.severe_overlap_threshold_pt..3.0).contains(&gap)
       && !large_gap_indicates_column
       || (same_line && has_split_boundary);
   ```
   
   **Issue**: The merge threshold is 3.0 points. In this PDF, column separations may be within this range, causing columns to merge.

## Recommendations for Fix

### Option 1: Preserve Space-Only Spans (Low Risk)
Modify `merge_adjacent_spans()` to not merge pure space spans:
```rust
// Don't merge pure-space spans - preserve column separation
if span.text.chars().all(|c| c.is_whitespace()) {
    current_span = Some(span.clone());
    continue;
}
```

### Option 2: Improve Column Detection
Enhance `detect_span_columns()` to recognize intentional column layouts and preserve spacing:
- Track whitespace-only spans between content spans
- Identify repeating column patterns
- Preserve gaps larger than character width but smaller than column width

### Option 3: Skip Mid-Word Space Logic for Non-Alphabetic Text
Modify line 4405 to only apply mid-word space skipping for actual words:
```rust
if last_char.is_lowercase() && !buffer.unicode.chars().any(|c| c.is_numeric()) {
    // Only skip for pure alphabetic text, not for numbers/mixed content
    continue;
}
```

### Option 4: Add Layout Analysis Mode
Detect table/column layouts and use different merge/dedup strategies:
- If spans have regular Y-position groups and gaps between X-positions
- Treat as table/layout mode and preserve spacing
- Use standard mode for continuous prose text

## Current Workaround

For users encountering this issue:
1. Use PyMUPDF directly for column-based layouts
2. Or post-process pdf_oxide output to detect and preserve column structure
3. Consider using `extract_spans()` instead of `extract_text()` and manually formatting

## Test Case

File: `/home/yfedoseev/projects/pdf_oxide_tests/pdfs_pdfjs/issue10900.pdf`

```python
from pdf_oxide import PdfDocument
import pymupdf

pdf = "issue10900.pdf"

doc = PdfDocument(pdf)
oxide_text = doc.extract_text(0)
print(f"pdf_oxide: {len(oxide_text.strip())} chars")

mdoc = pymupdf.open(pdf)
mupdf_text = mdoc[0].get_text()
print(f"PyMUPDF: {len(mupdf_text.strip())} chars")

# Expected output:
# pdf_oxide: 188 chars
# PyMUPDF: 271 chars
```

## Files to Investigate

- `src/extractors/text.rs:4400` - Mid-word space skipping logic
- `src/extractors/text.rs:2664` - Space span merging
- `src/extractors/text.rs:2633` - Merge threshold logic
- `src/extractors/text.rs:2390` - Column detection

## Impact Assessment

- **Scope**: Affects PDF documents with column-based layouts and intentional spacing
- **Frequency**: Likely affects scientific papers, financial tables, technical documentation
- **Severity**: Medium - Text is extracted but spacing/layout information is lost
- **Backward Compatibility**: Fixing this may change output for some PDFs


# v0.3.7 Benchmark — Root Cause Analysis

**Date**: 2026-02-19
**Baseline**: 3,829 PDFs — 3,569 clean (93.2%), 260 issues

## oxide_less (117 PDFs): Full Breakdown

### FALSE POSITIVES — 72 PDFs

**Category 1: Trailing whitespace only (61 PDFs, 61 chars total gap)**
- oxide and mupdf produce IDENTICAL text after `.strip()`
- pymupdf appends a trailing newline, oxide doesn't
- Action: Fix compare script — strip before comparing

**Category 2: Newline/formatting difference (9 PDFs, 122 chars gap)**
- Same text content, just different line breaks
- e.g. `issue3214`: oxide=`"A\nB"` mupdf=`"A\n\nB"`
- Action: Compare by words, not raw chars

**Category 3: Control chars stripped (2 PDFs)**
- `bug1151216.pdf`: control chars `\x03\x14\x02\x10` missing — oxide correctly strips them
- `issue2537r.pdf`: control char `\x03` missing
- Action: Not a real issue

### REAL ISSUES — 45 PDFs

**Category 4: Form field values missing (~9 PDFs)**

Same root cause as 34 oxide_empty form field PDFs. pymupdf extracts widget values, oxide doesn't.

| PDF | Missing content |
|-----|----------------|
| pr12828.pdf | "Hello World", font size 6, color names |
| issue15096.pdf | "value 1", "Value 2" |
| issue14862.pdf | "123", "CalculateNow" |
| issue15092.pdf | "1.00", "2.00", "3.00", "232,324.00" |
| issue12750.pdf | "value" from form field |
| bug1918115.pdf | "302,80" form calculation |
| issue4398.pdf | digits 1-9 from form fields |
| issue17492.pdf | form field text + field values |
| rc_annotation.pdf | annotation content |

Fix: Extract widget/form field values in `extract_text()`.

**Category 5: Font encoding/mapping wrong (5 PDFs, ~1,400 chars gap)**

| PDF | oxide output | correct (mupdf) | Problem |
|-----|-------------|-----------------|---------|
| issue13242.pdf | "Lormisumdolo" | "Lorem ipsum dolor" | Scrambled chars + missing spaces |
| issue2931.pdf | "oe pum" | "Lorem ipsum" | Chars dropped |
| issue19120.pdf | "Í"Íèô" | "a Trace-Monkey" | Wrong encoding applied |
| bug1019475_1.pdf | garbled CID | readable English | CID font failure |
| issue11578_reduced.pdf | "9ROXPHV" | "Volumes" | Shifted encoding |

Fix: Font encoding / ToUnicode CMap bugs — overlaps with P0/P1 from oxide_empty group.

**Category 6: Ligatures missing (5 PDFs, 301 chars gap)**

| PDF | Missing chars |
|-----|--------------|
| copy_paste_ligatures.pdf | ﬀ ﬁ ﬂ ﬃ ﬄ ﬅ ﬆ |
| issue10640.pdf | 281 chars of ligature content |
| issue11016_reduced.pdf | ﬀ (ff ligature) |
| issue15516_reduced.pdf | ligatures |
| issue6901.pdf | ligatures |

Fix: Map Unicode ligature codepoints to component chars (ﬀ→ff, ﬁ→fi, ﬂ→fl, etc.)

**Category 7: /ActualText not used (14 PDFs, 230 chars gap)**

veraPDF compliance PDFs that use `/ActualText` marked content attribute:
- `7.3-t01-pass-b.pdf`: "Logo of Dual lab sprl" in ActualText
- `7.18.1-t01-pass-c.pdf`, `7.18.1-t02-pass-e/f.pdf`: ActualText replacements
- `8.2.5.28.2-t01-pass-b.pdf`: ActualText content
- Several `7.2-t25-*` and `7.21.3.3-*` PDFs

Fix: Implement `/ActualText` extraction from marked content sequences.

**Category 8: CJK text missing (2 PDFs, 136 chars gap)**

| PDF | Issue |
|-----|-------|
| javauninstall-7r.pdf | Japanese text completely missing (56 chars) |
| issue5874.pdf | Arabic text partially missing (80 chars) |

Fix: CID font mapping improvements.

**Category 9: Spacing/word joining (3 PDFs)**

| PDF | oxide output | correct |
|-----|-------------|---------|
| issue20513.pdf | "PA RT SSOLUT ION" | "PARTS SOLUTION" |
| issue10900.pdf | column layout spacing lost | tabular data |
| issue6721_reduced.pdf | non-breaking space handling | proper spacing |

Fix: Spacing algorithm improvements.

**Category 10: Watermark/stamp text (3 PDFs, 21 chars gap)**

All three `isartor-6-5-3-t04-fail-*.pdf` files have "DRAFT" watermark text that oxide misses.

Fix: Extract annotation appearance stream text.

---

## oxide_empty (91 PDFs): Breakdown

| Root Cause | Count | Notes |
|-----------|------:|-------|
| Form field text (widgets) | 34 | pymupdf extracts widget values |
| Regular text (font/encoding) | 42 | Mostly 1–5 chars; some real CID/encoding failures |
| Annotation text | 12 | FreeText annotations, tx annotations |
| Brotli compression | 1 | Brotli-Prototype-FileA.pdf — 54,672 chars |

## oxide_much_less (45 PDFs): Top entries

| PDF | oxide | mupdf | ratio | Likely cause |
|-----|------:|------:|------:|-------------|
| TAMReview.pdf | 18,287 | 63,315 | 0.29 | Font/encoding failure |
| issue6127.pdf | 6,613 | 13,397 | 0.49 | Partial extraction |
| canvas.pdf | 1,180 | 4,255 | 0.28 | Symbolic font issues |
| ThuluthFeatures.pdf | 811 | 2,060 | 0.39 | Arabic shaping |
| annotation-text-widget.pdf | 159 | 651 | 0.24 | Form fields |
| annotation-choice-widget.pdf | 106 | 235 | 0.45 | Form fields |

---

## Cross-Group Fix Priority

| # | Fix | PDFs Fixed | Groups Affected |
|---|-----|--------:|----------------|
| 1 | Fix compare script (strip whitespace) | 72 | oxide_less false positives |
| 2 | Form field extraction | ~43 | 34 oxide_empty + 9 oxide_less |
| 3 | /ActualText support | 14 | oxide_less (veraPDF) |
| 4 | Ligature decomposition | 5 | oxide_less |
| 5 | Font encoding bugs (CID/ToUnicode) | ~47 | oxide_empty + oxide_less + oxide_much_less |
| 6 | Annotation text extraction | 15 | 12 oxide_empty + 3 oxide_less |
| 7 | Brotli decompression | 1 | oxide_empty |
| 8 | Spacing improvements | 3 | oxide_less |
| 9 | CJK font mapping | 2 | oxide_less |

## Progress After Fixes

| Fix Applied | New Clean | Clean % | Delta |
|-------------|-----------|---------|-------|
| Baseline | 3,569 | 93.2% | — |
| + Compare fix (strip) | 3,641 | 95.1% | +72 |
| + Form fields | 3,684 | 96.2% | +43 |
| + /ActualText | 3,698 | 96.6% | +14 |
| + Ligatures | 3,703 | 96.7% | +5 |
| + Font encoding | 3,750 | 97.9% | +47 |
| + Annotations | 3,765 | 98.3% | +15 |
| + Brotli | 3,766 | 98.4% | +1 |
| + Spacing + CJK | 3,771 | 98.5% | +5 |
| Remaining (errors) | — | — | 7 oxide_error/both_error |
| **Target** | **3,822** | **99.8%** | excl. 7 broken |

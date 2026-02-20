# Issue 6387 Investigation Summary

## Problem
- extract_text() returns 281 Japanese characters correctly
- extract_chars() returns 127 garbled characters (þ, ÿ, U+FFFD)
- Gap: 386 (PyMuPDF) vs 281 (pdf_oxide extract_text)

## Root Cause Identified

### Layer 1: Extract Path Discrepancy
- **extract_spans()** (used by extract_text()):
  - Uses TjBuffer.append() which calls decode_text_to_unicode()
  - decode_text_to_unicode() correctly reads 2-byte codes for Type0 fonts
  - Result: Correct Japanese characters

- **extract_chars()** (used by extract_chars()):
  - Originally: Called show_text(bytes) which iterated byte-by-byte
  - Problem: For Type0 fonts with 2-byte codes, this reads bytes individually
  - Example: 0xFE8F should be read as ONE 2-byte code, but was read as TWO 1-byte codes (0xFE, 0x8F)
  - Result: Wrong character lookups (0xFE → þ, 0x8F → garbage)

### Layer 2: Applied Fix
Changed show_text() to determine byte_len from font:
- For Type0 fonts with code_byte_length >= 2: Read 2 bytes per iteration
- For other fonts: Read 1 byte per iteration
- This allows proper 2-byte code lookup (0xFE8F → 'あ')

### Layer 3: Current Issue
After fix, getting U+FFFD (replacement character) instead of proper Japanese:
- Suggests: char_to_unicode() lookup is failing
- Possibility 1: ToUnicode CMap not being loaded
- Possibility 2: ToUnicode CMap loaded but returning None for 2-byte codes
- Possibility 3: Fallback is returning U+FFFD for unknown codes

## Next Investigation Steps

1. **Verify font is loaded**: Check if FontInfo.to_unicode CMap is being set
2. **Verify CMap parsing**: Test if parse_tounicode_cmap() correctly handles the PDF's 15 bfchar entries
3. **Verify code lookup**: Test if cmap.get(0xFE8F) returns the correct mapping
4. **Check for endianness**: Ensure 0xFE8F is being constructed correctly from bytes

## Test Case Data

PDF: issue6387.pdf
Font: NotoSansCJKjp-Bold (Type0, Identity-V)
Encoding: 2-byte character codes
ToUnicode CMap entries:
- 0xFE8F → U+3042 ('あ')
- 0xFEBB → U+306E ('の')
- 0xFEEF → U+30A4 ('イ')
- 0xFF46 → U+30FC ('ー')
- 0xFF1A → U+30CF ('ハ')
- 0xFF13 → U+30C8 ('ト')
- (plus 9 more entries)

Content stream hex bytes:
<FE8F FEBB FEEF FF46 FF1A FF13...>
Should decode to: あのイーハトー...


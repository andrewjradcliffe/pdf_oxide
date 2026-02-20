# Issue 6387: Japanese Vertical Text Character Extraction Gap

## Problem Summary
- pdf_oxide extracts 281 characters from issue6387.pdf using extract_text()
- PyMuPDF extracts 386 characters (37% more)
- extract_chars() returns garbled characters (þ, ÿ, U+FFFD) instead of proper Japanese
- extract_text() correctly returns Japanese characters

## Root Cause

### PDF Structure
- Font: NotoSansCJKjp-Bold (Type0/CID font)
- Encoding: Identity-V (vertical CJK)
- ToUnicode CMap: Yes (15 entries mapping font codes to Japanese)

### ToUnicode Mappings (from PDF)
```
<FE8F> <3042>  → U+3042 = 'あ'
<FEBB> <306E>  → U+306E = 'の'
<FEEF> <30A4>  → U+30A4 = 'イ'
<FF46> <30FC>  → U+30FC = 'ー'
... etc
```

### Content Stream
Uses 2-byte character codes in hex format:
```
<FE8F FEBB FEEF FF46 FF1A FF13 ...>  % Japanese text
```

### The Bug

**extract_text() flow:**
1. Calls extract_spans()
2. TjBuffer.append() uses decode_text_to_unicode()
3. decode_text_to_unicode() correctly handles Type0 fonts:
   - Reads bytes as 2-byte codes (based on code_byte_length = 2)
   - Looks up each 2-byte code in ToUnicode CMap
   - Returns correct Japanese characters

**extract_chars() flow:**
1. Calls extract() which calls execute_operator()
2. Tj operator calls show_text(bytes)
3. show_text() **incorrectly** iterates byte-by-byte:
   ```rust
   for &byte in text {
       let char_code = byte as u16;  // <- Only reads 1 byte!
       font.char_to_unicode(char_code as u32)
   }
   ```
4. For 2-byte codes (0xFE8F):
   - First iteration: processes 0xFE → looks up as code 0x00FE → gets 'þ'
   - Second iteration: processes 0x8F → looks up as code 0x008F → gets garbage or fallback

## The Fix

Change show_text() to use decode_text_to_unicode() just like TjBuffer does:

```rust
// Before (buggy - byte-by-byte):
fn show_text(&mut self, text: &[u8]) -> Result<()> {
    for &byte in text {
        let char_code = byte as u16;
        let unicode_string = font.char_to_unicode(char_code as u32)...
    }
}

// After (fixed - multi-byte aware):
fn show_text(&mut self, text: &[u8]) -> Result<()> {
    // Use the same multi-byte decoding logic as TjBuffer
    let font = font_name.as_ref().and_then(|name| self.fonts.get(name));
    let unicode_string = decode_text_to_unicode(text, font);
    
    // Process each character in the decoded Unicode string
    for unicode_char in unicode_string.chars() {
        // Create TextChar with proper positioning
    }
}
```

## Impact
This fix should:
1. Extract all 386 characters (matching PyMuPDF)
2. Show proper Japanese characters in extract_chars()
3. Not break existing tests (all other fonts work at byte-level)
4. Fix vertical text extraction for Type0 fonts

# Fix Aliased &mut PdfDocument UB in TextExtractor

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Eliminate undefined behavior caused by overlapping `&mut PdfDocument` references during recursive XObject processing in `TextExtractor`.

**Architecture:** The `TextExtractor` stores a `*mut PdfDocument` raw pointer and dereferences it as `&mut` inside `process_xobject`. During nested Form XObject processing, `process_xobject` recurses through `execute_operator`, creating stacked `&mut PdfDocument` references that violate Rust's Stacked Borrows aliasing rules. Miri confirms the UB at `text.rs:4589`. Two fix tiers are implemented on separate branches.

**Tech Stack:** Rust, `RefCell`/`Cell` for interior mutability (Tier 2)

---

## Branch Structure

Both branches start from `main` and include the Miri-validated test file `tests/test_nested_xobject_aliasing.rs`.

- **Branch `fix/xobject-aliasing-tier1`** — Scope the borrows (~50-80 lines, `text.rs` only)
- **Branch `fix/xobject-aliasing-tier2`** — Interior mutability (~250-350 lines, `document.rs` + `text.rs`)

---

## Tier 1: Scope the Borrows

### Task 1: Create branch and add test

**Files:**
- Create: `tests/test_nested_xobject_aliasing.rs` (already written)

**Step 1: Create branch from main**

```bash
git checkout -b fix/xobject-aliasing-tier1 main
```

**Step 2: Stage and commit the test file**

```bash
git add tests/test_nested_xobject_aliasing.rs
git commit -m "test: add Miri-validated test for nested XObject aliasing UB"
```

**Step 3: Verify Miri detects the UB**

```bash
rustup run nightly cargo miri test --test test_nested_xobject_aliasing test_nested_xobject_extraction_does_not_segfault
```

Expected: FAIL with `Undefined Behavior: trying to retag from <...> for SharedReadWrite permission`

---

### Task 2: Restructure `process_xobject` to scope `doc` borrows

**Files:**
- Modify: `src/extractors/text.rs:4376-4600`

The key change: split `process_xobject` so that `doc: &mut PdfDocument` is **never held across the recursive `parse_and_execute_text_only` call** (line 4571). This eliminates overlapping `&mut` references during recursion.

**Step 1: Identify the three phases**

Current code holds `doc` (line 4402) across the entire function through line 4589. Split into:

- **Phase 1** (lines 4402-4563): Acquire `doc`, do all cache checks, load stream data, load fonts. Collect results into local variables. Drop `doc` at end of block.
- **Phase 2** (lines 4569-4572): Recursive `parse_and_execute_text_only`. No `doc` reference exists.
- **Phase 3** (lines 4583-4589): Re-acquire `doc` from raw pointer to update `xobject_spans_cache`.

**Step 2: Implement the restructuring**

Wrap Phase 1 in a block that returns the data needed for Phase 2 and 3:

```rust
// Phase 1: gather data from document (doc borrow scoped to this block)
let (stream_data, has_own_resources, saved_fonts, saved_resources, saved_xobj_cache, spans_before) = {
    let doc = match self.document {
        Some(ptr) => unsafe { &mut *ptr },
        None => return Ok(()),
    };

    // ... all existing cache checks, is_form_xobject, load_object,
    // decode_stream_with_encryption, load_fonts ...
    // ... collect results into local variables ...

    (stream_data, has_own_resources, saved_fonts, saved_resources, saved_xobj_cache, spans_before)
};
// doc is dropped here

// Phase 2: recursive parse (no doc reference held)
self.xobject_depth += 1;
let parse_result =
    parse_and_execute_text_only(&stream_data, |op| self.execute_operator(op));
self.xobject_depth -= 1;
if let Err(e) = parse_result {
    log::debug!("Error parsing Form XObject '{}': {}", name, e);
}

// Phase 3: re-acquire doc for cache update (short-lived, non-overlapping)
if has_own_resources && self.extract_spans {
    let new_spans = if self.spans.len() > spans_before {
        Some(self.spans[spans_before..].to_vec())
    } else {
        None
    };
    if let Some(ptr) = self.document {
        let doc = unsafe { &mut *ptr };
        doc.xobject_spans_cache.insert(xobject_ref, new_spans);
    }
}

// Restore fonts, resources, and XObject cache
if let Some(fonts) = saved_fonts {
    self.fonts = fonts;
}
// ... rest of restore logic ...
```

Key considerations:
- `stream_data` must be moved out of Phase 1 (it's a `Vec<u8>` or `Arc<Vec<u8>>` clone)
- `saved_fonts`, `saved_resources`, `saved_xobj_cache` are already local — just hoist them out of the block
- The early returns inside Phase 1 (e.g., `doc.xobject_text_free_cache.insert(); return Ok(())`) work fine inside the block — return the data as an `Option` or use a helper enum to signal early exit

**Step 3: Handle early returns**

Phase 1 has multiple early return points (no-text XObject, image XObject, cached results, etc.). Use a local enum or `Option` wrapper:

```rust
enum Phase1Result {
    EarlyReturn,
    Continue {
        stream_data: Vec<u8>,
        has_own_resources: bool,
        saved_fonts: Option<HashMap<String, Arc<FontInfo>>>,
        saved_resources: Option<Object>,
        saved_xobj_cache: Option<HashMap<String, Option<ObjectRef>>>,
        spans_before: usize,
    },
}
```

**Step 4: Run tests**

```bash
cargo test --test test_nested_xobject_aliasing
```

Expected: all 3 tests PASS

**Step 5: Run Miri**

```bash
rustup run nightly cargo miri test --test test_nested_xobject_aliasing test_nested_xobject_extraction_does_not_segfault
```

Expected: PASS (no UB detected)

**Step 6: Run full test suite**

```bash
cargo test
```

Expected: no regressions

**Step 7: Commit**

```bash
git add src/extractors/text.rs
git commit -m "fix: scope &mut PdfDocument borrows in process_xobject to eliminate UB

Restructure process_xobject into three phases so that the &mut PdfDocument
reference is never held across the recursive parse_and_execute_text_only call.
This eliminates overlapping &mut references during nested XObject processing,
fixing undefined behavior confirmed by Miri (Stacked Borrows violation)."
```

---

## Tier 2: Interior Mutability

### Task 3: Create branch from main

**Step 1: Create branch**

```bash
git checkout -b fix/xobject-aliasing-tier2 main
```

**Step 2: Cherry-pick the test commit from Tier 1**

```bash
git cherry-pick <test-commit-sha>
```

---

### Task 4: Wrap PdfDocument cache fields in RefCell/Cell

**Files:**
- Modify: `src/document.rs:108-189` (struct definition)
- Modify: `src/document.rs:390-433` (constructor)

**Step 1: Change field types**

Wrap these fields in `RefCell` (or `Cell` for `Copy` types):

| Field | Current Type | New Type |
|-------|-------------|----------|
| `reader` | `PdfReader` | `RefCell<PdfReader>` |
| `object_cache` | `HashMap<ObjectRef, Object>` | `RefCell<HashMap<ObjectRef, Object>>` |
| `scanned_object_offsets` | `Option<HashMap<u32, u64>>` | `RefCell<Option<HashMap<u32, u64>>>` |
| `image_xobject_cache` | `HashSet<ObjectRef>` | `RefCell<HashSet<ObjectRef>>` |
| `font_cache` | `HashMap<ObjectRef, Arc<FontInfo>>` | `RefCell<HashMap<ObjectRef, Arc<FontInfo>>>` |
| `font_set_cache` | `HashMap<ObjectRef, Vec<...>>` | `RefCell<HashMap<ObjectRef, Vec<...>>>` |
| `font_fingerprint_cache` | `HashMap<u64, Vec<...>>` | `RefCell<HashMap<u64, Vec<...>>>` |
| `font_name_set_cache` | `HashMap<u64, (...)>` | `RefCell<HashMap<u64, (...)>>` |
| `font_identity_cache` | `HashMap<u64, Arc<FontInfo>>` | `RefCell<HashMap<u64, Arc<FontInfo>>>` |
| `xobject_text_free_cache` | `HashSet<ObjectRef>` | `RefCell<HashSet<ObjectRef>>` |
| `xobject_stream_cache` | `HashMap<ObjectRef, Arc<Vec<u8>>>` | `RefCell<HashMap<ObjectRef, Arc<Vec<u8>>>>` |
| `xobject_stream_cache_bytes` | `usize` | `Cell<usize>` |
| `xobject_spans_cache` | `HashMap<ObjectRef, Option<Vec<TextSpan>>>` | `RefCell<HashMap<ObjectRef, Option<Vec<TextSpan>>>>` |

This follows the existing pattern: `resolving_stack` and `recursion_depth` already use `RefCell`.

**Step 2: Update constructor to wrap in RefCell::new/Cell::new**

**Step 3: Update Debug impl (line 196)**

Change `self.object_cache.len()` → `self.object_cache.borrow().len()`

**Step 4: Commit**

```bash
git add src/document.rs
git commit -m "refactor: wrap PdfDocument cache fields in RefCell/Cell for interior mutability"
```

---

### Task 5: Change method signatures to `&self`

**Files:**
- Modify: `src/document.rs` — multiple methods

**Step 1: Change these methods from `&mut self` to `&self`:**

- `load_object` (line 831) — update all `self.object_cache` to `.borrow()`/`.borrow_mut()`, `self.reader` to `.borrow_mut()` for seek/read
- `scan_for_object` (line 712) — update `self.reader` and `self.scanned_object_offsets`
- `load_uncompressed_object` (line 1158) + `_impl` (line 1163)
- `load_compressed_object` (line 1396)
- `is_form_xobject` (line 1089) — update `self.image_xobject_cache` to `.borrow()`/`.borrow_mut()`
- `load_fonts` (line 6675) — update font cache accesses

**Step 2: Update all field accesses in these methods**

Pattern: `self.object_cache.get(...)` → `self.object_cache.borrow().get(...)` (for reads)
Pattern: `self.object_cache.insert(...)` → `self.object_cache.borrow_mut().insert(...)` (for writes)
Pattern: `self.reader.seek(...)` → `self.reader.borrow_mut().seek(...)` (for I/O)

**Important:** Keep `borrow()` / `borrow_mut()` scopes as tight as possible. Never hold a `RefMut` across a method call that might also borrow the same RefCell. For example:

```rust
// BAD: holds borrow across load_object call
let cache = self.object_cache.borrow();
let obj = self.load_object(ref)?; // load_object also borrows object_cache → panic

// GOOD: drop borrow before calling
let cached = self.object_cache.borrow().get(&ref).cloned();
if let Some(obj) = cached { return Ok(obj); }
```

**Step 3: Update callers of these methods that passed `&mut self`**

Since `&self` is a subset of `&mut self`, callers with `&mut self` don't need changes. Only callers that explicitly needed `&mut self` solely because of these methods might now be relaxable, but this is optional cleanup.

**Step 4: Update other direct accesses to wrapped fields**

Search for all remaining `self.object_cache`, `self.image_xobject_cache`, etc. and add `.borrow()`/`.borrow_mut()` as needed. Be careful about methods that still take `&mut self` — they can still use `.borrow_mut()`, it just wasn't necessary before.

**Step 5: Compile and fix errors iteratively**

```bash
cargo check 2>&1 | head -50
```

Fix each error. The compiler will guide you to every access site that needs updating.

**Step 6: Commit**

```bash
git add src/document.rs
git commit -m "refactor: change load_object, is_form_xobject, load_fonts to &self

Use RefCell::borrow/borrow_mut for cache and reader access, enabling
these methods to be called through a shared reference."
```

---

### Task 6: Change TextExtractor pointer from `*mut` to `*const`

**Files:**
- Modify: `src/extractors/text.rs:1781,1946-1957,2237,2245,2253,4333,4402`
- Modify: `src/document.rs:4286,5037,5139,5228`

**Step 1: Change the field type**

```rust
// text.rs line 1781
// OLD:
document: Option<*mut crate::document::PdfDocument>,
// NEW:
document: Option<*const crate::document::PdfDocument>,
```

**Step 2: Update set_document and set_document_ptr**

```rust
// text.rs line 1946
pub fn set_document(&mut self, document: *const crate::document::PdfDocument) {
    self.document = Some(document);
}

// text.rs line 1955
pub fn set_document_ptr(&mut self, doc: &crate::document::PdfDocument) {
    self.document = Some(doc as *const crate::document::PdfDocument);
}
```

**Step 3: Change all dereferences from `&mut *ptr` to `&*ptr`**

5 sites in text.rs:
- Line 2237: `let doc = unsafe { &*self.document? };`
- Line 2245: `let doc = unsafe { &*self.document? };`
- Line 2253: `let doc = unsafe { &*self.document? };`
- Line 4333: `let doc = unsafe { &*doc_ptr };`
- Line 4402: `let doc = unsafe { &*ptr };` (variable name may differ)

**Step 4: Change all xobject cache accesses from direct field to `.borrow()`/`.borrow_mut()`**

In `process_xobject` (text.rs), all direct field accesses like:
- `doc.xobject_text_free_cache.contains(...)` → `doc.xobject_text_free_cache.borrow().contains(...)`
- `doc.xobject_text_free_cache.insert(...)` → `doc.xobject_text_free_cache.borrow_mut().insert(...)`
- `doc.xobject_spans_cache.get(...)` → `doc.xobject_spans_cache.borrow().get(...).cloned()`
- `doc.xobject_spans_cache.insert(...)` → `doc.xobject_spans_cache.borrow_mut().insert(...)`
- `doc.xobject_stream_cache.get(...)` → `doc.xobject_stream_cache.borrow().get(...).cloned()`
- `doc.xobject_stream_cache.insert(...)` → `doc.xobject_stream_cache.borrow_mut().insert(...)`
- `doc.xobject_stream_cache_bytes` → `doc.xobject_stream_cache_bytes.get()` / `.set(...)`

**Step 5: Update caller sites in document.rs**

4 sites where `set_document` is called:
- Line 4286: `extractor.set_document(self as *const PdfDocument);`
- Line 5037: same
- Line 5139: same
- Line 5228: same

**Step 6: Compile and fix**

```bash
cargo check 2>&1 | head -50
```

**Step 7: Commit**

```bash
git add src/extractors/text.rs src/document.rs
git commit -m "fix: change TextExtractor document pointer from *mut to *const

Now that PdfDocument methods use interior mutability (RefCell), the
extractor only needs a shared reference. This eliminates all aliased
&mut PdfDocument references, making the code fully sound under
Stacked Borrows."
```

---

### Task 7: Validate Tier 2

**Step 1: Run tests**

```bash
cargo test --test test_nested_xobject_aliasing
```

Expected: all 3 tests PASS

**Step 2: Run Miri**

```bash
rustup run nightly cargo miri test --test test_nested_xobject_aliasing test_nested_xobject_extraction_does_not_segfault
```

Expected: PASS (no UB detected)

**Step 3: Run full test suite**

```bash
cargo test
```

Expected: no regressions

**Step 4: Run clippy**

```bash
cargo clippy -- -D warnings
```

Expected: no new warnings

---

## Verification Checklist

For both branches:
- [ ] `cargo test` passes (no regressions)
- [ ] `cargo miri test --test test_nested_xobject_aliasing` passes (UB eliminated)
- [ ] `cargo clippy -- -D warnings` clean
- [ ] `cargo test --release` passes (no optimization-dependent crashes)

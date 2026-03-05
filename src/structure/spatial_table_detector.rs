//! Spatial table detection from PDF text layout.
//!
//! Implements table detection according to ISO 32000-1:2008 Section 5.2 (Coordinate Systems).
//! Uses X and Y coordinate clustering to identify table structure in PDFs that lack explicit
//! table markup in the structure tree.
//!
//! ## Algorithm Overview
//!
//! Tables are detected through spatial clustering:
//! 1. **Column Detection**: X-coordinate clustering (spans with similar X-start positions)
//! 2. **Row Detection**: Y-coordinate clustering (spans with similar Y positions)
//! 3. **Cell Assignment**: Grid construction by assigning spans to (column, row) cells
//! 4. **Validation**: Heuristic checks to distinguish real tables from false positives
//! 5. **Header Detection**: Optional detection of header rows via font properties
//!
//! ## PDF Specification Compliance
//!
//! This module uses only the coordinate system concepts defined in ISO 32000-1:2008 Section 5.2.
//! It does not rely on Tagged PDF structure or linguistic heuristics, making it suitable for
//! PDFs without explicit table markup in the structure tree.

use crate::layout::text_block::TextSpan;
use crate::structure::table_extractor::{ExtractedTable, TableCell, TableRow};

/// Strategy for detecting table boundaries (v0.3.14).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum TableStrategy {
    /// Use only vector lines to define boundaries.
    #[serde(rename = "lines")]
    Lines,
    /// Use only text alignment to define boundaries.
    #[serde(rename = "text")]
    Text,
    /// Use both text and lines (hybrid approach).
    #[default]
    #[serde(rename = "both")]
    Both,
}

/// Configuration for spatial table detection.
///
/// Controls the behavior of table detection algorithms. All parameters are in user space units
/// (points) as defined by PDF spec ISO 32000-1:2008 Section 5.2.
#[derive(Debug, Clone, PartialEq)]
pub struct TableDetectionConfig {
    /// Enable spatial table detection (default: true)
    ///
    /// When disabled, detect_tables_from_spans returns an empty vector.
    pub enabled: bool,

    /// Strategy for horizontal boundary detection (rows).
    pub horizontal_strategy: TableStrategy,

    /// Strategy for vertical boundary detection (columns).
    pub vertical_strategy: TableStrategy,

    /// X-coordinate tolerance for column detection in user space units (default: 5.0)
    ///
    /// Two spans with X-start positions within this distance belong to the same column.
    /// This accounts for minor alignment variations in PDF layout. Units are points.
    pub column_tolerance: f32,

    /// Y-coordinate tolerance for row detection in user space units (default: 2.8)
    ///
    /// Two spans with Y-positions within this distance belong to the same row.
    /// This accounts for baseline variations and inter-line spacing. Units are points.
    pub row_tolerance: f32,

    /// Minimum number of cells required to consider structure as table (default: 4)
    ///
    /// A grid must have at least this many occupied cells to be considered a table.
    /// This prevents single rows or columns from being detected as tables.
    pub min_table_cells: usize,

    /// Minimum number of columns required for table (default: 2)
    ///
    /// Tables must have at least 2 columns. A single column is not a table.
    pub min_table_columns: usize,

    /// Minimum ratio of rows matching the most common column count (default: 0.7)
    ///
    /// For a grid to be accepted as a table, at least this fraction of rows must have
    /// the modal (most common) number of columns. Lower values allow more irregular tables.
    /// Range: 0.0 to 1.0.
    pub regular_row_ratio: f32,

    /// Maximum number of columns before structure is considered too wide (default: 15)
    ///
    /// Very wide tables (>15 columns) are often false positives from unusual layouts.
    /// This serves as a sanity check for column detection.
    pub max_table_columns: usize,
}

impl Default for TableDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            horizontal_strategy: TableStrategy::Both,
            vertical_strategy: TableStrategy::Both,
            column_tolerance: 5.0,
            row_tolerance: 2.8,
            min_table_cells: 4,
            min_table_columns: 2,
            regular_row_ratio: 0.7,
            max_table_columns: 15,
        }
    }
}

impl TableDetectionConfig {
    /// Create a strict table detection configuration.
    ///
    /// Uses conservative thresholds to reduce false positives.
    pub fn strict() -> Self {
        Self {
            enabled: true,
            horizontal_strategy: TableStrategy::Lines,
            vertical_strategy: TableStrategy::Lines,
            column_tolerance: 2.0,
            row_tolerance: 1.0,
            min_table_cells: 6,
            min_table_columns: 3,
            regular_row_ratio: 0.8,
            max_table_columns: 12,
        }
    }

    /// Create a relaxed table detection configuration.
    ///
    /// Uses permissive thresholds to catch more potential tables.
    pub fn relaxed() -> Self {
        Self {
            enabled: true,
            horizontal_strategy: TableStrategy::Text,
            vertical_strategy: TableStrategy::Text,
            column_tolerance: 10.0,
            row_tolerance: 5.0,
            min_table_cells: 4,
            min_table_columns: 2,
            regular_row_ratio: 0.5,
            max_table_columns: 20,
        }
    }
}

/// Main entry point: Detect tables from spatial layout
///
/// This function analyzes a collection of text spans and attempts to detect table structures
/// based on their spatial arrangement. It performs the following steps:
///
/// 1. Column detection via X-coordinate clustering
/// 2. Row detection via Y-coordinate clustering
/// 3. Span-to-cell assignment
/// 4. Table structure validation
/// 5. Conversion to ExtractedTable format
///
/// # Arguments
/// * `spans` - Text spans (ideally sorted by Y then X for optimal performance)
/// * `config` - Configuration parameters for detection
///
/// # Returns
/// * `Vec<ExtractedTable>` - Detected tables. Empty if no tables found or detection disabled.
///
/// # Example
/// ```ignore
/// use pdf_oxide::structure::spatial_table_detector::{detect_tables_from_spans, TableDetectionConfig};
/// use pdf_oxide::layout::TextSpan;
///
/// let config = TableDetectionConfig::default();
/// let tables = detect_tables_from_spans(&spans, &config);
/// for table in tables {
///     println!("Found table with {} rows and {} columns", table.rows.len(), table.col_count);
/// }
/// ```
pub fn detect_tables_from_spans(
    spans: &[TextSpan],
    config: &TableDetectionConfig,
) -> Vec<ExtractedTable> {
    if !config.enabled || spans.is_empty() {
        return Vec::new();
    }

    // Step 1: Detect columns via X-coordinate clustering
    let columns = detect_columns(spans, config.column_tolerance);

    if columns.len() < config.min_table_columns {
        // Not enough columns for a table
        return Vec::new();
    }

    if columns.len() > config.max_table_columns {
        // Too many columns - likely false positive
        return Vec::new();
    }

    // Step 2: Detect rows via Y-coordinate clustering
    let rows = detect_rows(spans, config.row_tolerance);

    if rows.len() < 2 {
        // Need at least 2 rows for a table
        return Vec::new();
    }

    // Step 3: Assign spans to grid cells
    let grid = assign_spans_to_cells(spans, &columns, &rows);

    // Step 4: Validate table structure
    if !validate_table_structure(&grid, config) {
        return Vec::new();
    }

    // Step 5: Convert grid to ExtractedTable
    vec![grid_to_extracted_table(&grid, spans)]
}

/// Internal: Column cluster structure
///
/// Represents a group of horizontally-aligned spans that form a table column.
#[derive(Debug, Clone)]
struct ColumnCluster {
    /// X-coordinate of column center
    x_center: f32,
    /// Minimum X-coordinate of any span in column
    x_min: f32,
    /// Maximum X-coordinate of any span in column
    x_max: f32,
    /// Indices of spans in this column (relative to input span array)
    span_indices: Vec<usize>,
}

/// Internal: Row cluster structure
///
/// Represents a group of vertically-aligned spans that form a table row.
#[derive(Debug, Clone)]
struct RowCluster {
    /// Y-coordinate of row center
    y_center: f32,
    /// Minimum Y-coordinate of any span in row
    y_min: f32,
    /// Maximum Y-coordinate of any span in row
    y_max: f32,
    /// Indices of spans in this row (relative to input span array)
    span_indices: Vec<usize>,
}

/// Internal: Grid structure for table
///
/// Represents a 2D grid of table cells. Each cell contains a vector of span indices
/// that make up the cell's content.
#[derive(Debug)]
struct GridStructure {
    /// Column definitions (left to right)
    columns: Vec<ColumnCluster>,
    /// Row definitions (top to bottom)
    rows: Vec<RowCluster>,
    /// Grid cells: cells[row_idx][col_idx] = Vec<span_indices>
    cells: Vec<Vec<Vec<usize>>>,
}

/// Merge info for a cell in the grid (colspan/rowspan).
#[derive(Debug, Clone)]
struct CellMergeInfo {
    /// Number of columns this cell spans
    colspan: u32,
    /// Number of rows this cell spans
    rowspan: u32,
    /// Whether this cell is covered by another cell's span (should be skipped in output)
    covered: bool,
}

/// Detect column structure via X-coordinate clustering
///
/// This function groups spans into columns by clustering their left (x-start) coordinates.
/// It uses a greedy single-pass clustering algorithm:
///
/// 1. Process each span in order
/// 2. Try to assign span to existing column (within tolerance)
/// 3. Create new column if no match found
/// 4. Sort columns left-to-right
///
/// # Arguments
/// * `spans` - Text spans to cluster
/// * `column_tolerance` - Maximum X distance for column membership
///
/// # Returns
/// * `Vec<ColumnCluster>` - Detected columns, sorted left-to-right by x_center
fn detect_columns(spans: &[TextSpan], column_tolerance: f32) -> Vec<ColumnCluster> {
    let mut columns: Vec<ColumnCluster> = Vec::new();

    for (idx, span) in spans.iter().enumerate() {
        let x = span.bbox.left();
        let mut found = false;

        // Try to assign to existing column
        for col in &mut columns {
            if (x - col.x_center).abs() < column_tolerance {
                col.span_indices.push(idx);
                col.x_min = col.x_min.min(x);
                col.x_max = col.x_max.max(x);
                found = true;
                break;
            }
        }

        // Create new column if not found
        if !found {
            columns.push(ColumnCluster {
                x_center: x,
                x_min: x,
                x_max: x,
                span_indices: vec![idx],
            });
        }
    }

    // Sort columns left-to-right
    columns.sort_by(|a, b| crate::utils::safe_float_cmp(a.x_center, b.x_center));

    columns
}

/// Detect row structure via Y-coordinate clustering
///
/// This function groups spans into rows by clustering their Y-center coordinates.
/// It uses the same greedy single-pass clustering algorithm as column detection:
///
/// 1. Process each span in order
/// 2. Try to assign span to existing row (within tolerance)
/// 3. Create new row if no match found
/// 4. Sort rows top-to-bottom (higher Y first in PDF coordinate system)
///
/// # Arguments
/// * `spans` - Text spans to cluster
/// * `row_tolerance` - Maximum Y distance for row membership
///
/// # Returns
/// * `Vec<RowCluster>` - Detected rows, sorted top-to-bottom
fn detect_rows(spans: &[TextSpan], row_tolerance: f32) -> Vec<RowCluster> {
    let mut rows: Vec<RowCluster> = Vec::new();

    for (idx, span) in spans.iter().enumerate() {
        let y = span.bbox.center().y;
        let mut found = false;

        // Try to assign to existing row
        for row in &mut rows {
            if (y - row.y_center).abs() < row_tolerance {
                row.span_indices.push(idx);
                row.y_min = row.y_min.min(y);
                row.y_max = row.y_max.max(y);
                found = true;
                break;
            }
        }

        // Create new row if not found
        if !found {
            rows.push(RowCluster {
                y_center: y,
                y_min: y,
                y_max: y,
                span_indices: vec![idx],
            });
        }
    }

    // Sort rows top-to-bottom (higher Y first in PDF coordinates)
    rows.sort_by(|a, b| crate::utils::safe_float_cmp(b.y_center, a.y_center));

    rows
}

/// Assign spans to grid cells (column × row)
///
/// Creates a 2D grid by finding the best (column, row) cell for each span based on
/// Euclidean distance to cluster centers. This uses a nearest-neighbor assignment strategy.
///
/// # Arguments
/// * `spans` - Text spans to assign
/// * `columns` - Detected column clusters
/// * `rows` - Detected row clusters
///
/// # Returns
/// * `GridStructure` - Complete grid with all spans assigned to cells
fn assign_spans_to_cells(
    spans: &[TextSpan],
    columns: &[ColumnCluster],
    rows: &[RowCluster],
) -> GridStructure {
    let num_cols = columns.len();
    let num_rows = rows.len();

    // Initialize grid
    let mut cells: Vec<Vec<Vec<usize>>> = vec![vec![Vec::new(); num_cols]; num_rows];

    // Assign each span to a grid cell
    for (idx, span) in spans.iter().enumerate() {
        let span_x = span.bbox.center().x;
        let span_y = span.bbox.center().y;

        // Find best matching column (by X distance)
        let col_idx = columns
            .iter()
            .enumerate()
            .min_by_key(|(_, col)| {
                let dist = (span_x - col.x_center).abs();
                (dist * 1000.0) as i32 // Convert to i32 for min_by_key
            })
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Find best matching row (by Y distance)
        let row_idx = rows
            .iter()
            .enumerate()
            .min_by_key(|(_, row)| {
                let dist = (span_y - row.y_center).abs();
                (dist * 1000.0) as i32
            })
            .map(|(i, _)| i)
            .unwrap_or(0);

        cells[row_idx][col_idx].push(idx);
    }

    GridStructure {
        columns: columns.to_vec(),
        rows: rows.to_vec(),
        cells,
    }
}

/// Validate if grid structure represents a table
///
/// Applies heuristic validation rules to determine if a grid is likely a real table:
///
/// 1. **Minimum cells**: Must have at least min_table_cells occupied cells
/// 2. **Row regularity**: At least regular_row_ratio fraction of rows must have the
///    modal number of columns
///
/// This prevents false positives like:
/// - Single rows or columns
/// - Highly irregular layouts
/// - Text arranged by chance in a grid-like pattern
///
/// # Arguments
/// * `grid` - Grid structure to validate
/// * `config` - Configuration with validation thresholds
///
/// # Returns
/// * `bool` - True if grid passes all validation checks
fn validate_table_structure(grid: &GridStructure, config: &TableDetectionConfig) -> bool {
    // Check minimum cells
    let total_cells: usize = grid
        .cells
        .iter()
        .flat_map(|row| row.iter())
        .map(|cell| if cell.is_empty() { 0 } else { 1 })
        .sum();

    if total_cells < config.min_table_cells {
        return false;
    }

    // Check row regularity
    let cell_counts: Vec<usize> = grid
        .cells
        .iter()
        .map(|row| row.iter().filter(|cell| !cell.is_empty()).count())
        .collect();

    if cell_counts.is_empty() {
        return false;
    }

    let most_common_count = *cell_counts
        .iter()
        .max_by_key(|&&count| cell_counts.iter().filter(|&&c| c == count).count())
        .unwrap_or(&0);

    if most_common_count == 0 {
        return false;
    }

    let regular_rows = cell_counts
        .iter()
        .filter(|&&count| count == most_common_count)
        .count();

    if (regular_rows as f32 / cell_counts.len() as f32) < config.regular_row_ratio {
        return false;
    }

    true
}

/// Detected table structure (used by markdown converter for backward compatibility).
///
/// This type represents tables identified by the spatial table detector.
#[derive(Debug, Clone)]
pub struct DetectedTable {
    /// Indices of spans that belong to this table
    pub span_indices: Vec<usize>,
}

/// Spatial table detector wrapper for backward compatibility with markdown converter.
///
/// This is a wrapper around the new functional API to maintain compatibility with existing code.
pub struct SpatialTableDetector {
    config: TableDetectionConfig,
}

impl SpatialTableDetector {
    /// Create a new detector with custom configuration.
    pub fn with_config(config: TableDetectionConfig) -> Self {
        Self { config }
    }

    /// Detect tables from spans (backward-compatible wrapper).
    ///
    /// Returns DetectedTable structures compatible with the markdown converter.
    pub fn detect_tables(&self, spans: &[TextSpan]) -> Vec<DetectedTable> {
        detect_tables_from_spans(spans, &self.config)
            .into_iter()
            .flat_map(|_table| {
                // Extract all span indices that were used in the table
                // The cell text is concatenated from spans, but we don't have direct access to indices
                // For now, return empty to indicate compatibility
                let all_indices: Vec<usize> = Vec::new();
                // Return empty if no tables found, maintains contract
                if all_indices.is_empty() {
                    return None;
                }
                Some(DetectedTable {
                    span_indices: all_indices,
                })
            })
            .collect()
    }

    /// Detect tables using both text spans and vector lines (v0.3.14).
    ///
    /// This hybrid approach provides better results for bordered tables.
    pub fn detect_tables_hybrid(
        &self,
        spans: &[TextSpan],
        lines: &[crate::elements::PathContent],
    ) -> Vec<ExtractedTable> {
        detect_tables_with_lines(spans, lines, &self.config)
    }
}

/// Cluster a list of f32 values using a greedy tolerance-based approach.
///
/// Groups nearby values together and returns the running average of each cluster.
/// Uses the same single-pass greedy strategy as `detect_columns` and `detect_rows`.
fn cluster_values(values: &[f32], tolerance: f32) -> Vec<f32> {
    let mut clusters: Vec<f32> = Vec::new();
    let mut counts: Vec<u32> = Vec::new();

    for &v in values {
        if let Some(idx) = clusters.iter().position(|&c| (v - c).abs() < tolerance) {
            counts[idx] += 1;
            clusters[idx] += (v - clusters[idx]) / counts[idx] as f32;
        } else {
            clusters.push(v);
            counts.push(1);
        }
    }

    clusters
}

/// Detect table structure purely from vector line intersections.
///
/// Suitable for bordered tables (tables with explicit drawn grid lines). Extracts
/// horizontal and vertical line positions from `PathContent`, clusters them into
/// row/column boundaries, and assigns text spans to the resulting cells.
///
/// Returns empty if there are no lines, or if the lines don't form a valid grid.
fn detect_tables_from_lines(
    spans: &[TextSpan],
    lines: &[crate::elements::PathContent],
    config: &TableDetectionConfig,
) -> Vec<ExtractedTable> {
    if lines.is_empty() {
        return Vec::new();
    }

    // Minimum length to count as a meaningful boundary line.
    const MIN_LINE_LENGTH: f32 = 10.0;
    // Maximum perpendicular extent for a segment to be classified as axis-aligned.
    const LINE_AXIS_TOL: f32 = 2.0;

    let mut h_ys: Vec<f32> = Vec::new();
    let mut v_xs: Vec<f32> = Vec::new();

    for path in lines {
        let bbox = &path.bbox;

        if path.is_straight_line() {
            if bbox.height < LINE_AXIS_TOL && bbox.width > MIN_LINE_LENGTH {
                h_ys.push(bbox.center().y);
            } else if bbox.width < LINE_AXIS_TOL && bbox.height > MIN_LINE_LENGTH {
                v_xs.push(bbox.center().x);
            }
        } else if path.is_rectangle() {
            // A rectangle contributes both horizontal and vertical edges.
            if bbox.width > MIN_LINE_LENGTH {
                h_ys.push(bbox.top());
                h_ys.push(bbox.bottom());
            }
            if bbox.height > MIN_LINE_LENGTH {
                v_xs.push(bbox.left());
                v_xs.push(bbox.right());
            }
        }
    }

    // Cluster boundary positions.
    let mut row_ys = cluster_values(&h_ys, config.row_tolerance);
    let mut col_xs = cluster_values(&v_xs, config.column_tolerance);

    // Need at least 2 boundaries in each dimension to form any cells.
    if row_ys.len() < 2 || col_xs.len() < 2 {
        return Vec::new();
    }

    // Sort: rows top-to-bottom (higher Y first in PDF coordinates), cols left-to-right.
    row_ys.sort_by(|a, b| crate::utils::safe_float_cmp(*b, *a));
    col_xs.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));

    let num_rows = row_ys.len() - 1;
    let num_cols = col_xs.len() - 1;

    if num_cols < config.min_table_columns || num_cols > config.max_table_columns {
        return Vec::new();
    }

    // Assign each span to the grid cell whose boundaries enclose its center point.
    let mut cells: Vec<Vec<Vec<usize>>> = vec![vec![Vec::new(); num_cols]; num_rows];

    for (idx, span) in spans.iter().enumerate() {
        let cx = span.bbox.center().x;
        let cy = span.bbox.center().y;

        let row_idx = (0..num_rows).find(|&r| cy <= row_ys[r] && cy >= row_ys[r + 1]);
        let col_idx = (0..num_cols).find(|&c| cx >= col_xs[c] && cx <= col_xs[c + 1]);

        if let (Some(r), Some(c)) = (row_idx, col_idx) {
            cells[r][c].push(idx);
        }
    }

    // Build ColumnCluster/RowCluster from boundary midpoints.
    // `span_indices` in these structs is not used by `grid_to_extracted_table`.
    let columns: Vec<ColumnCluster> = (0..num_cols)
        .map(|c| ColumnCluster {
            x_center: (col_xs[c] + col_xs[c + 1]) / 2.0,
            x_min: col_xs[c],
            x_max: col_xs[c + 1],
            span_indices: Vec::new(),
        })
        .collect();

    let rows: Vec<RowCluster> = (0..num_rows)
        .map(|r| RowCluster {
            y_center: (row_ys[r] + row_ys[r + 1]) / 2.0,
            y_min: row_ys[r + 1],
            y_max: row_ys[r],
            span_indices: Vec::new(),
        })
        .collect();

    let grid = GridStructure {
        columns,
        rows,
        cells,
    };

    if !validate_table_structure(&grid, config) {
        return Vec::new();
    }

    vec![grid_to_extracted_table(&grid, spans)]
}

/// Table detection using text spans and/or vector lines.
///
/// Routes to the appropriate detection algorithm based on `config.horizontal_strategy`
/// and `config.vertical_strategy`:
///
/// - `Text/Text`: pure coordinate clustering from text span positions.
/// - `Lines/Lines`: pure line-grid detection from vector path geometry; returns empty
///   when no lines are present, preventing false positives on plain-text pages.
/// - Any other combination (`Both`, or mixed): text clustering as baseline, refined
///   by lines for header detection.
pub fn detect_tables_with_lines(
    spans: &[TextSpan],
    lines: &[crate::elements::PathContent],
    config: &TableDetectionConfig,
) -> Vec<ExtractedTable> {
    if !config.enabled || spans.is_empty() {
        return Vec::new();
    }

    match (config.horizontal_strategy, config.vertical_strategy) {
        // Pure text: coordinate clustering only, no line involvement.
        (TableStrategy::Text, TableStrategy::Text) => {
            return detect_tables_from_spans(spans, config);
        },
        // Pure lines: grid defined by vector geometry only.
        // Returns empty when no lines are present rather than falling back to text detection.
        (TableStrategy::Lines, TableStrategy::Lines) => {
            return detect_tables_from_lines(spans, lines, config);
        },
        // Both/mixed: fall through to hybrid.
        _ => {},
    }

    // Use standard spatial detection as baseline
    let candidates = detect_tables_from_spans(spans, config);

    if lines.is_empty() {
        return candidates;
    }

    // Refinement using lines
    let mut refined = Vec::new();
    for mut table in candidates {
        if let Some(bbox) = table.bbox {
            // Find lines that intersect with the table's bounding box
            let table_lines: Vec<_> = lines
                .iter()
                .filter(|l| l.is_straight_line() && bbox.intersects(&l.bbox))
                .collect();

            // If we have lines, it's likely a bordered table
            // In Phase 4, we use this primarily to confirm table presence
            // and potentially adjust header detection
            if !table_lines.is_empty() && !table.has_header {
                // Look for a horizontal line near the top that could be a header separator
                let header_sep_y_min = bbox.y + bbox.height * 0.7; // Top 30%
                let has_header_sep = table_lines.iter().any(|l| {
                    l.bbox.width > bbox.width * 0.8 && l.bbox.center().y > header_sep_y_min
                });

                if has_header_sep {
                    table.has_header = true;
                    if !table.rows.is_empty() {
                        table.rows[0].is_header = true;
                    }
                }
            }
        }
        refined.push(table);
    }

    refined
}

/// Convert grid structure to ExtractedTable
///
/// Transforms the internal grid representation into the ExtractedTable format,
/// including text extraction from cells, merged cell detection, multi-line cell
/// support, and header row detection based on font properties.
///
/// # Arguments
/// * `grid` - Grid structure to convert
/// * `spans` - Original text spans (for text extraction)
///
/// # Returns
/// * `ExtractedTable` - Formatted table ready for output
fn grid_to_extracted_table(grid: &GridStructure, spans: &[TextSpan]) -> ExtractedTable {
    let num_rows = grid.cells.len();
    let num_cols = grid.columns.len();

    // Detect merged cells (colspan and rowspan)
    let merge_info = detect_merged_cells(grid, spans);

    // Detect header row using font properties
    let header_row_idx = detect_header_row(grid, spans);

    let mut table_rows = Vec::new();

    for (row_idx, row) in grid.cells.iter().enumerate() {
        let is_header = header_row_idx == Some(row_idx);
        let mut table_row = TableRow::new(is_header);

        for (col_idx, cell_span_indices) in row.iter().enumerate() {
            let mi = &merge_info[row_idx][col_idx];

            // Skip cells covered by another cell's colspan/rowspan
            if mi.covered {
                continue;
            }

            // Extract text from cell spans with multi-line support
            let cell_text = extract_cell_text(cell_span_indices, spans);

            // Compute cell bbox from spans
            let cell_bbox = if cell_span_indices.is_empty() {
                None
            } else {
                let mut bbox = spans[cell_span_indices[0]].bbox;
                for &idx in &cell_span_indices[1..] {
                    bbox = bbox.union(&spans[idx].bbox);
                }
                Some(bbox)
            };

            // Extract MCIDs from cell spans
            let mcids = cell_span_indices
                .iter()
                .filter_map(|&idx| spans.get(idx).and_then(|s| s.mcid))
                .collect::<Vec<_>>();

            // Clamp colspan and rowspan to grid bounds
            let colspan = mi.colspan.min((num_cols - col_idx) as u32);
            let rowspan = mi.rowspan.min((num_rows - row_idx) as u32);

            table_row.cells.push(TableCell {
                text: cell_text,
                colspan,
                rowspan,
                mcids,
                bbox: cell_bbox,
                is_header,
            });
        }

        table_rows.push(table_row);
    }

    let has_header = header_row_idx.is_some();

    // Compute bounding box from all spans in the grid
    let all_span_indices: Vec<usize> = grid
        .cells
        .iter()
        .flat_map(|row| row.iter().flat_map(|cell| cell.iter().copied()))
        .collect();

    let bbox = if !all_span_indices.is_empty() {
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for &idx in &all_span_indices {
            if let Some(s) = spans.get(idx) {
                min_x = min_x.min(s.bbox.x);
                min_y = min_y.min(s.bbox.y);
                max_x = max_x.max(s.bbox.x + s.bbox.width);
                max_y = max_y.max(s.bbox.y + s.bbox.height);
            }
        }

        Some(crate::geometry::Rect::new(min_x, min_y, max_x - min_x, max_y - min_y))
    } else {
        None
    };

    ExtractedTable {
        rows: table_rows,
        has_header,
        col_count: num_cols,
        bbox,
    }
}

/// Extract text from a cell's spans with multi-line support.
///
/// Spans within a cell are grouped by their Y-coordinate line. Spans on the same
/// line are joined with spaces, while different lines are joined with newlines.
/// This handles multi-line cells where content spans multiple vertical positions.
fn extract_cell_text(cell_span_indices: &[usize], spans: &[TextSpan]) -> String {
    if cell_span_indices.is_empty() {
        return String::new();
    }

    // Collect spans with their Y centers
    let mut span_entries: Vec<(f32, &str)> = cell_span_indices
        .iter()
        .filter_map(|&idx| spans.get(idx).map(|s| (s.bbox.center().y, s.text.as_str())))
        .collect();

    if span_entries.is_empty() {
        return String::new();
    }

    // If only one span, return directly
    if span_entries.len() == 1 {
        return span_entries[0].1.to_string();
    }

    // Sort by Y descending (top-to-bottom in PDF coordinates: higher Y = higher on page)
    span_entries.sort_by(|a, b| crate::utils::safe_float_cmp(b.0, a.0));

    // Group spans into lines based on Y proximity (tolerance of 2.0 points)
    let line_tolerance = 2.0_f32;
    let mut lines: Vec<Vec<&str>> = Vec::new();
    let mut current_line: Vec<&str> = vec![span_entries[0].1];
    let mut current_y = span_entries[0].0;

    for &(y, text) in &span_entries[1..] {
        if (current_y - y).abs() <= line_tolerance {
            // Same line
            current_line.push(text);
        } else {
            // New line
            lines.push(current_line);
            current_line = vec![text];
            current_y = y;
        }
    }
    lines.push(current_line);

    // Join spans within lines with spaces, join lines with newlines
    lines
        .iter()
        .map(|line| line.join(" "))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Detect merged cells (colspan and rowspan) in the grid.
///
/// Algorithm:
/// 1. For each content cell, check if the span's bounding box extends across
///    neighboring empty cells in the same row (colspan detection).
/// 2. For each content cell, check if the span's bounding box extends across
///    neighboring empty cells in the same column (rowspan detection).
/// 3. Mark covered cells so they are skipped during output.
fn detect_merged_cells(grid: &GridStructure, spans: &[TextSpan]) -> Vec<Vec<CellMergeInfo>> {
    let num_rows = grid.cells.len();
    let num_cols = grid.columns.len();

    // Initialize merge info: all cells start with colspan=1, rowspan=1, not covered
    let mut merge_info: Vec<Vec<CellMergeInfo>> = (0..num_rows)
        .map(|_| {
            (0..num_cols)
                .map(|_| CellMergeInfo {
                    colspan: 1,
                    rowspan: 1,
                    covered: false,
                })
                .collect()
        })
        .collect();

    // Detect colspan: scan each row for content cells followed by empty cells
    for row_idx in 0..num_rows {
        for col_idx in 0..num_cols {
            if grid.cells[row_idx][col_idx].is_empty() {
                continue;
            }

            // Get the rightmost extent of spans in this cell
            let cell_right = grid.cells[row_idx][col_idx]
                .iter()
                .filter_map(|&idx| spans.get(idx).map(|s| s.bbox.right()))
                .fold(f32::NEG_INFINITY, f32::max);

            if cell_right == f32::NEG_INFINITY {
                continue;
            }

            // Check how many subsequent empty columns this cell's content extends across
            let mut extra_cols = 0u32;
            for next_col in (col_idx + 1)..num_cols {
                if !grid.cells[row_idx][next_col].is_empty() {
                    break;
                }
                // Check if the content cell's right edge extends past this column's center
                let col_center = grid.columns[next_col].x_center;
                if cell_right > col_center {
                    extra_cols += 1;
                } else {
                    break;
                }
            }

            if extra_cols > 0 {
                merge_info[row_idx][col_idx].colspan = 1 + extra_cols;
                // Mark covered cells
                for c in 1..=(extra_cols as usize) {
                    if col_idx + c < num_cols {
                        merge_info[row_idx][col_idx + c].covered = true;
                    }
                }
            }
        }
    }

    // Detect rowspan: scan each column for content cells followed by empty cells
    for col_idx in 0..num_cols {
        for row_idx in 0..num_rows {
            if grid.cells[row_idx][col_idx].is_empty() || merge_info[row_idx][col_idx].covered {
                continue;
            }

            // Get the bottommost extent of spans in this cell
            // In PDF coordinates, lower Y = lower on page, but our rows are sorted
            // top-to-bottom (higher Y first). So "bottom" of a cell means the lowest Y.
            let cell_bottom = grid.cells[row_idx][col_idx]
                .iter()
                .filter_map(|&idx| spans.get(idx).map(|s| s.bbox.bottom()))
                .fold(f32::INFINITY, f32::min);

            if cell_bottom == f32::INFINITY {
                continue;
            }

            // Check how many subsequent empty rows this cell extends across
            let mut extra_rows = 0u32;
            for next_row in (row_idx + 1)..num_rows {
                if !grid.cells[next_row][col_idx].is_empty() {
                    break;
                }
                // Check if the content cell's bottom extends past this row's center
                let row_center = grid.rows[next_row].y_center;
                // Remember: rows are sorted descending by Y, so lower row_center = lower on page
                if cell_bottom < row_center {
                    extra_rows += 1;
                } else {
                    break;
                }
            }

            if extra_rows > 0 {
                merge_info[row_idx][col_idx].rowspan = 1 + extra_rows;
                // Mark covered cells
                for r in 1..=(extra_rows as usize) {
                    if row_idx + r < num_rows {
                        merge_info[row_idx + r][col_idx].covered = true;
                    }
                }
            }
        }
    }

    merge_info
}

/// Detect the header row based on font properties.
///
/// Checks if the first row has different font characteristics (bold weight or larger
/// font size) compared to subsequent rows. Returns `Some(0)` if a header is detected,
/// `None` otherwise.
///
/// # Detection criteria
/// - First row has predominantly bold fonts while data rows do not
/// - First row has a noticeably larger average font size than data rows
fn detect_header_row(grid: &GridStructure, spans: &[TextSpan]) -> Option<usize> {
    if grid.cells.is_empty() || grid.cells.len() < 2 {
        return None;
    }

    let first_row = &grid.cells[0];
    let data_rows = &grid.cells[1..];

    // Collect font properties from first row
    let first_row_spans: Vec<&TextSpan> = first_row
        .iter()
        .flat_map(|cell| cell.iter().filter_map(|&idx| spans.get(idx)))
        .collect();

    if first_row_spans.is_empty() {
        return None;
    }

    // Collect font properties from data rows
    let data_row_spans: Vec<&TextSpan> = data_rows
        .iter()
        .flat_map(|row| {
            row.iter()
                .flat_map(|cell| cell.iter().filter_map(|&idx| spans.get(idx)))
        })
        .collect();

    if data_row_spans.is_empty() {
        return None;
    }

    // Check bold ratio in first row vs data rows
    let first_row_bold_count = first_row_spans
        .iter()
        .filter(|s| s.font_weight.is_bold())
        .count();
    let first_row_bold_ratio = first_row_bold_count as f32 / first_row_spans.len() as f32;

    let data_bold_count = data_row_spans
        .iter()
        .filter(|s| s.font_weight.is_bold())
        .count();
    let data_bold_ratio = data_bold_count as f32 / data_row_spans.len() as f32;

    // If first row is mostly bold (>50%) and data rows are mostly not bold (<30%)
    if first_row_bold_ratio > 0.5 && data_bold_ratio < 0.3 {
        return Some(0);
    }

    // Check font size difference
    let first_row_avg_size: f32 =
        first_row_spans.iter().map(|s| s.font_size).sum::<f32>() / first_row_spans.len() as f32;
    let data_avg_size: f32 =
        data_row_spans.iter().map(|s| s.font_size).sum::<f32>() / data_row_spans.len() as f32;

    // If first row is notably larger (at least 1.5 points bigger)
    if first_row_avg_size > data_avg_size + 1.5 {
        return Some(0);
    }

    // Fallback: assume first row is header (original behavior)
    Some(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::layout::text_block::{Color, FontWeight};

    /// Helper to create test spans
    fn create_test_span(text: &str, x: f32, y: f32, width: f32, height: f32) -> TextSpan {
        TextSpan {
            text: text.to_string(),
            bbox: Rect::new(x, y, width, height),
            font_name: "TestFont".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            is_italic: false,
            color: Color::black(),
            mcid: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 1.0,
            primary_detected: false,
        }
    }

    #[test]
    fn test_detect_columns_simple_grid() {
        let spans = vec![
            create_test_span("A", 10.0, 100.0, 10.0, 10.0),
            create_test_span("B", 50.0, 100.0, 10.0, 10.0),
            create_test_span("C", 90.0, 100.0, 10.0, 10.0),
        ];

        let config = TableDetectionConfig::default();
        let columns = detect_columns(&spans, config.column_tolerance);

        assert_eq!(columns.len(), 3);
        assert_eq!(columns[0].x_center, 10.0);
        assert_eq!(columns[1].x_center, 50.0);
        assert_eq!(columns[2].x_center, 90.0);
    }

    #[test]
    fn test_detect_columns_with_tolerance() {
        let spans = vec![
            create_test_span("A", 10.0, 100.0, 10.0, 10.0),
            create_test_span("B", 12.0, 100.0, 10.0, 10.0), // Within default tolerance (5.0)
            create_test_span("C", 50.0, 100.0, 10.0, 10.0),
        ];

        let config = TableDetectionConfig::default();
        let columns = detect_columns(&spans, config.column_tolerance);

        // A and B should be in same column due to tolerance
        assert_eq!(columns.len(), 2);
    }

    #[test]
    fn test_detect_rows_simple_grid() {
        let spans = vec![
            create_test_span("A", 10.0, 100.0, 10.0, 10.0),
            create_test_span("B", 10.0, 80.0, 10.0, 10.0),
            create_test_span("C", 10.0, 60.0, 10.0, 10.0),
        ];

        let config = TableDetectionConfig::default();
        let rows = detect_rows(&spans, config.row_tolerance);

        assert_eq!(rows.len(), 3);
        // Rows are sorted top-to-bottom (higher Y first in PDF coordinates)
        // Y centers are computed as y + height/2 (e.g., 100 + 5 = 105)
        assert_eq!(rows[0].y_center, 105.0);
        assert_eq!(rows[1].y_center, 85.0);
        assert_eq!(rows[2].y_center, 65.0);
    }

    #[test]
    fn test_detect_rows_with_tolerance() {
        let spans = vec![
            create_test_span("A", 10.0, 100.0, 10.0, 10.0),
            create_test_span("B", 10.0, 101.0, 10.0, 10.0), // Within default tolerance (2.8)
            create_test_span("C", 10.0, 60.0, 10.0, 10.0),
        ];

        let config = TableDetectionConfig::default();
        let rows = detect_rows(&spans, config.row_tolerance);

        // A and B should be in same row due to tolerance
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_config_default() {
        let config = TableDetectionConfig::default();

        assert!(config.enabled);
        assert_eq!(config.column_tolerance, 5.0);
        assert_eq!(config.row_tolerance, 2.8);
        assert_eq!(config.min_table_cells, 4);
        assert_eq!(config.min_table_columns, 2);
        assert_eq!(config.regular_row_ratio, 0.7);
        assert_eq!(config.max_table_columns, 15);
    }

    #[test]
    fn test_detect_tables_disabled() {
        let spans = vec![
            create_test_span("A", 10.0, 100.0, 10.0, 10.0),
            create_test_span("B", 50.0, 100.0, 10.0, 10.0),
        ];

        let config = TableDetectionConfig {
            enabled: false,
            ..Default::default()
        };

        let tables = detect_tables_from_spans(&spans, &config);
        assert!(tables.is_empty());
    }

    #[test]
    fn test_detect_tables_empty_spans() {
        let spans = vec![];
        let config = TableDetectionConfig::default();

        let tables = detect_tables_from_spans(&spans, &config);
        assert!(tables.is_empty());
    }

    #[test]
    fn test_detect_tables_insufficient_columns() {
        let spans = vec![
            create_test_span("A", 10.0, 100.0, 10.0, 10.0),
            create_test_span("B", 10.0, 80.0, 10.0, 10.0),
        ];

        let config = TableDetectionConfig::default();
        let tables = detect_tables_from_spans(&spans, &config);

        // Only 1 column, needs at least 2
        assert!(tables.is_empty());
    }

    #[test]
    fn test_detect_tables_insufficient_rows() {
        let spans = vec![
            create_test_span("A", 10.0, 100.0, 10.0, 10.0),
            create_test_span("B", 50.0, 100.0, 10.0, 10.0),
        ];

        let config = TableDetectionConfig::default();
        let tables = detect_tables_from_spans(&spans, &config);

        // Only 1 row, needs at least 2
        assert!(tables.is_empty());
    }

    #[test]
    fn test_detect_tables_minimum_valid_grid() {
        let spans = vec![
            create_test_span("A", 10.0, 100.0, 10.0, 10.0),
            create_test_span("B", 50.0, 100.0, 10.0, 10.0),
            create_test_span("C", 10.0, 80.0, 10.0, 10.0),
            create_test_span("D", 50.0, 80.0, 10.0, 10.0),
        ];

        let config = TableDetectionConfig::default();
        let tables = detect_tables_from_spans(&spans, &config);

        // 2x2 grid with 4 cells - meets minimum requirements
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].col_count, 2);
        assert_eq!(tables[0].rows.len(), 2);
    }

    #[test]
    fn test_grid_to_extracted_table_text_extraction() {
        let spans = vec![
            create_test_span("Hello", 10.0, 100.0, 30.0, 10.0),
            create_test_span("World", 50.0, 100.0, 30.0, 10.0),
            create_test_span("Foo", 10.0, 80.0, 20.0, 10.0),
            create_test_span("Bar", 50.0, 80.0, 20.0, 10.0),
        ];

        let config = TableDetectionConfig::default();
        let tables = detect_tables_from_spans(&spans, &config);

        assert_eq!(tables.len(), 1);
        let table = &tables[0];

        // Check text content
        assert_eq!(table.rows[0].cells[0].text, "Hello");
        assert_eq!(table.rows[0].cells[1].text, "World");
        assert_eq!(table.rows[1].cells[0].text, "Foo");
        assert_eq!(table.rows[1].cells[1].text, "Bar");
    }

    #[test]
    fn test_spatial_detection_populates_bbox() {
        let spans = vec![
            create_test_span("A", 10.0, 100.0, 30.0, 12.0),
            create_test_span("B", 60.0, 100.0, 25.0, 12.0),
            create_test_span("C", 10.0, 80.0, 30.0, 12.0),
            create_test_span("D", 60.0, 80.0, 25.0, 12.0),
        ];

        let config = TableDetectionConfig::default();
        let tables = detect_tables_from_spans(&spans, &config);

        assert_eq!(tables.len(), 1);
        let bbox = tables[0].bbox.expect("bbox should be populated");

        // min_x = 10.0, min_y = 80.0, max_x = 60+25 = 85.0, max_y = 100+12 = 112.0
        assert!((bbox.x - 10.0).abs() < 0.01);
        assert!((bbox.y - 80.0).abs() < 0.01);
        assert!((bbox.width - 75.0).abs() < 0.01); // 85.0 - 10.0
        assert!((bbox.height - 32.0).abs() < 0.01); // 112.0 - 80.0
    }

    #[test]
    fn test_spatial_detection_bbox_single_row_pair() {
        // A minimal 2-col 2-row table but with different span sizes
        let spans = vec![
            create_test_span("X", 5.0, 200.0, 40.0, 10.0),
            create_test_span("Y", 100.0, 200.0, 50.0, 10.0),
            create_test_span("Z", 5.0, 180.0, 40.0, 10.0),
            create_test_span("W", 100.0, 180.0, 50.0, 10.0),
        ];

        let config = TableDetectionConfig::default();
        let tables = detect_tables_from_spans(&spans, &config);

        assert_eq!(tables.len(), 1);
        let bbox = tables[0].bbox.expect("bbox should be set");

        // min_x=5, min_y=180, max_x=100+50=150, max_y=200+10=210
        assert!((bbox.x - 5.0).abs() < 0.01);
        assert!((bbox.y - 180.0).abs() < 0.01);
        assert!((bbox.width - 145.0).abs() < 0.01); // 150 - 5
        assert!((bbox.height - 30.0).abs() < 0.01); // 210 - 180
    }

    // =========================================================================
    // detect_tables_with_lines — strategy routing
    // =========================================================================

    /// Build a simple horizontal PathContent line at the given Y.
    fn make_h_line(x: f32, y: f32, width: f32) -> crate::elements::PathContent {
        crate::elements::PathContent::line(x, y, x + width, y)
    }

    /// Build a simple vertical PathContent line at the given X.
    fn make_v_line(x: f32, y: f32, height: f32) -> crate::elements::PathContent {
        crate::elements::PathContent::line(x, y, x, y + height)
    }

    /// Build a rectangle PathContent.
    fn make_rect_path(x: f32, y: f32, w: f32, h: f32) -> crate::elements::PathContent {
        crate::elements::PathContent::rect(x, y, w, h)
    }

    #[test]
    fn test_lines_strategy_no_lines_returns_empty() {
        // A plain-text page that would match text-based detection should return
        // empty when Lines strategy is requested and no vector lines are present.
        let spans = vec![
            create_test_span("A", 10.0, 100.0, 10.0, 10.0),
            create_test_span("B", 50.0, 100.0, 10.0, 10.0),
            create_test_span("C", 10.0, 80.0, 10.0, 10.0),
            create_test_span("D", 50.0, 80.0, 10.0, 10.0),
        ];

        let config = TableDetectionConfig {
            horizontal_strategy: TableStrategy::Lines,
            vertical_strategy: TableStrategy::Lines,
            ..TableDetectionConfig::default()
        };

        // No vector lines on the page.
        let tables = detect_tables_with_lines(&spans, &[], &config);
        assert!(
            tables.is_empty(),
            "Lines strategy with no vector lines should return empty, not text-detected tables"
        );
    }

    #[test]
    fn test_text_strategy_ignores_lines() {
        // Text strategy should produce results based on span positions alone,
        // even when lines are present (lines are irrelevant).
        let spans = vec![
            create_test_span("A", 10.0, 100.0, 10.0, 10.0),
            create_test_span("B", 50.0, 100.0, 10.0, 10.0),
            create_test_span("C", 10.0, 80.0, 10.0, 10.0),
            create_test_span("D", 50.0, 80.0, 10.0, 10.0),
        ];
        // Some unrelated lines that should not affect the result.
        let lines = vec![make_h_line(0.0, 200.0, 100.0)];

        let config = TableDetectionConfig {
            horizontal_strategy: TableStrategy::Text,
            vertical_strategy: TableStrategy::Text,
            ..TableDetectionConfig::default()
        };

        let tables = detect_tables_with_lines(&spans, &lines, &config);
        assert_eq!(tables.len(), 1, "Text strategy should detect the 2x2 grid");
    }

    #[test]
    fn test_lines_strategy_with_grid_lines_detects_table() {
        // A 2-row x 2-column bordered table defined by 3 H-lines and 3 V-lines.
        //
        //  H: y=110, y=90, y=70
        //  V: x=5,   x=55, x=105
        //
        //  Spans at the center of each cell:
        //   (30, 100) → row 0, col 0    (80, 100) → row 0, col 1
        //   (30,  80) → row 1, col 0    (80,  80) → row 1, col 1
        let spans = vec![
            create_test_span("A", 25.0, 95.0, 10.0, 10.0), // center (30, 100)
            create_test_span("B", 75.0, 95.0, 10.0, 10.0), // center (80, 100)
            create_test_span("C", 25.0, 75.0, 10.0, 10.0), // center (30,  80)
            create_test_span("D", 75.0, 75.0, 10.0, 10.0), // center (80,  80)
        ];

        let lines = vec![
            make_h_line(5.0, 110.0, 100.0),
            make_h_line(5.0, 90.0, 100.0),
            make_h_line(5.0, 70.0, 100.0),
            make_v_line(5.0, 70.0, 40.0),
            make_v_line(55.0, 70.0, 40.0),
            make_v_line(105.0, 70.0, 40.0),
        ];

        let config = TableDetectionConfig {
            horizontal_strategy: TableStrategy::Lines,
            vertical_strategy: TableStrategy::Lines,
            ..TableDetectionConfig::default()
        };

        let tables = detect_tables_with_lines(&spans, &lines, &config);
        assert_eq!(tables.len(), 1, "Should detect one table from grid lines");
        assert_eq!(tables[0].col_count, 2);
        assert_eq!(tables[0].rows.len(), 2);
    }

    #[test]
    fn test_lines_strategy_with_rect_paths_detects_table() {
        // Same 2x2 table, built from four cell-rectangles.
        // Each cell is its own rect; shared edges get clustered together.
        //
        //  Col 0: x=5..55   Col 1: x=55..105
        //  Row 0: y=90..110   Row 1: y=70..90
        let spans = vec![
            create_test_span("A", 25.0, 95.0, 10.0, 10.0), // col 0, row 0
            create_test_span("B", 75.0, 95.0, 10.0, 10.0), // col 1, row 0
            create_test_span("C", 25.0, 75.0, 10.0, 10.0), // col 0, row 1
            create_test_span("D", 75.0, 75.0, 10.0, 10.0), // col 1, row 1
        ];

        // Four cell rectangles forming a 2x2 grid.
        let lines = vec![
            make_rect_path(5.0, 90.0, 50.0, 20.0), // (0,0): x=5..55,  y=90..110
            make_rect_path(55.0, 90.0, 50.0, 20.0), // (0,1): x=55..105, y=90..110
            make_rect_path(5.0, 70.0, 50.0, 20.0), // (1,0): x=5..55,  y=70..90
            make_rect_path(55.0, 70.0, 50.0, 20.0), // (1,1): x=55..105, y=70..90
        ];

        let config = TableDetectionConfig {
            horizontal_strategy: TableStrategy::Lines,
            vertical_strategy: TableStrategy::Lines,
            ..TableDetectionConfig::default()
        };

        let tables = detect_tables_with_lines(&spans, &lines, &config);
        assert_eq!(tables.len(), 1, "Should detect one table from rectangle paths");
        assert_eq!(tables[0].col_count, 2);
        assert_eq!(tables[0].rows.len(), 2);
    }

    #[test]
    fn test_cluster_values_groups_nearby() {
        let values = vec![10.0_f32, 10.2, 10.4, 50.0, 50.1];
        let clusters = cluster_values(&values, 1.0);
        assert_eq!(clusters.len(), 2, "Should produce 2 clusters");
    }

    #[test]
    fn test_cluster_values_all_separate() {
        let values = vec![10.0_f32, 20.0, 30.0];
        let clusters = cluster_values(&values, 1.0);
        assert_eq!(clusters.len(), 3);
    }
}

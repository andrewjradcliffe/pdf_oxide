//! Benchmark all slow PDF sets: measures text extraction, image extraction, and markdown conversion.
//!
//! Usage: bench_slow_pdfs [pdf_oxide_tests_dir]

use pdf_oxide::converters::ConversionOptions;
use pdf_oxide::document::PdfDocument;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let base = if args.len() > 1 {
        args[1].clone()
    } else {
        let home = std::env::var("HOME").unwrap();
        format!("{}/projects/pdf_oxide_tests", home)
    };

    let slow_dirs: Vec<String> = (1..=6)
        .map(|i| {
            if i == 1 {
                format!("{}/pdfs_slow", base)
            } else {
                format!("{}/pdfs_slow{}", base, i)
            }
        })
        .collect();

    // Collect all PDFs
    let mut all_pdfs: Vec<(String, String)> = Vec::new(); // (set_name, path)
    for dir in &slow_dirs {
        let set_name = dir.rsplit('/').next().unwrap().to_string();
        if let Ok(entries) = collect_pdfs(dir) {
            for path in entries {
                all_pdfs.push((set_name.clone(), path));
            }
        }
    }

    eprintln!("Total PDFs to benchmark: {}", all_pdfs.len());
    eprintln!();

    let options = ConversionOptions {
        include_images: true,
        ..ConversionOptions::default()
    };

    let mut results: Vec<PdfResult> = Vec::new();
    let global_start = Instant::now();

    for (idx, (set_name, path)) in all_pdfs.iter().enumerate() {
        let filename = path.rsplit('/').next().unwrap_or(path);
        let short_name = if filename.len() > 60 {
            format!("{}...", &filename[..57])
        } else {
            filename.to_string()
        };

        eprint!("[{}/{}] {} ... ", idx + 1, all_pdfs.len(), short_name);

        let mut doc = match PdfDocument::open(path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("OPEN_ERROR: {}", e);
                results.push(PdfResult {
                    set: set_name.clone(),
                    name: filename.to_string(),
                    pages: 0,
                    text_ms: 0.0,
                    md_ms: 0.0,
                    img_ms: 0.0,
                    images_total: 0,
                    error: Some(format!("{}", e)),
                    slow_pages: Vec::new(),
                });
                continue;
            }
        };

        let page_count = doc.page_count().unwrap_or(0);

        // --- Text extraction ---
        let t = Instant::now();
        for p in 0..page_count {
            let _ = doc.extract_text(p);
        }
        let text_ms = t.elapsed().as_secs_f64() * 1000.0;

        // --- Image extraction ---
        let t = Instant::now();
        let mut images_total = 0usize;
        let mut img_page_times: Vec<(usize, f64, usize)> = Vec::new();
        for p in 0..page_count {
            let pt = Instant::now();
            match doc.extract_images(p) {
                Ok(imgs) => {
                    let pms = pt.elapsed().as_secs_f64() * 1000.0;
                    images_total += imgs.len();
                    if pms > 200.0 {
                        img_page_times.push((p, pms, imgs.len()));
                    }
                }
                Err(_) => {}
            }
        }
        let img_ms = t.elapsed().as_secs_f64() * 1000.0;

        // --- Markdown extraction (with images) ---
        let t = Instant::now();
        let mut md_page_times: Vec<(usize, f64)> = Vec::new();
        for p in 0..page_count {
            let pt = Instant::now();
            match doc.to_markdown(p, &options) {
                Ok(_md) => {
                    let pms = pt.elapsed().as_secs_f64() * 1000.0;
                    if pms > 200.0 {
                        md_page_times.push((p, pms));
                    }
                }
                Err(_) => {}
            }
        }
        let md_ms = t.elapsed().as_secs_f64() * 1000.0;

        // Collect slow pages
        let mut slow_pages = Vec::new();
        for (p, ms, cnt) in &img_page_times {
            slow_pages.push(format!("  img p{}: {:.0}ms ({} imgs)", p, ms, cnt));
        }
        for (p, ms) in &md_page_times {
            slow_pages.push(format!("  md  p{}: {:.0}ms", p, ms));
        }

        let total = text_ms + img_ms + md_ms;
        eprintln!(
            "{} pages | text={:.0}ms img={:.0}ms md={:.0}ms | total={:.1}s",
            page_count, text_ms, img_ms, md_ms, total / 1000.0
        );

        results.push(PdfResult {
            set: set_name.clone(),
            name: filename.to_string(),
            pages: page_count,
            text_ms,
            md_ms,
            img_ms,
            images_total,
            error: None,
            slow_pages,
        });
    }

    let global_elapsed = global_start.elapsed().as_secs_f64();

    // === Summary ===
    println!();
    println!("================================================================");
    println!("  BENCHMARK RESULTS  —  {} PDFs in {:.1}s", results.len(), global_elapsed);
    println!("================================================================");

    // Per-set summary
    let mut sets: Vec<String> = results.iter().map(|r| r.set.clone()).collect();
    sets.sort();
    sets.dedup();

    for set in &sets {
        let set_results: Vec<&PdfResult> = results.iter().filter(|r| r.set == *set).collect();
        let count = set_results.len();
        let errors = set_results.iter().filter(|r| r.error.is_some()).count();
        let total_text: f64 = set_results.iter().map(|r| r.text_ms).sum();
        let total_img: f64 = set_results.iter().map(|r| r.img_ms).sum();
        let total_md: f64 = set_results.iter().map(|r| r.md_ms).sum();
        let total_pages: usize = set_results.iter().map(|r| r.pages).sum();
        println!(
            "\n  {} ({} PDFs, {} pages, {} errors):",
            set, count, total_pages, errors
        );
        println!(
            "    text: {:>7.1}s  img: {:>7.1}s  md: {:>7.1}s  total: {:>7.1}s",
            total_text / 1000.0,
            total_img / 1000.0,
            total_md / 1000.0,
            (total_text + total_img + total_md) / 1000.0
        );
        println!(
            "    mean/PDF: text={:.0}ms img={:.0}ms md={:.0}ms",
            total_text / count as f64,
            total_img / count as f64,
            total_md / count as f64
        );
    }

    // Global totals
    let total_text: f64 = results.iter().map(|r| r.text_ms).sum();
    let total_img: f64 = results.iter().map(|r| r.img_ms).sum();
    let total_md: f64 = results.iter().map(|r| r.md_ms).sum();
    let total_pages: usize = results.iter().map(|r| r.pages).sum();
    let total_images: usize = results.iter().map(|r| r.images_total).sum();
    println!("\n  GLOBAL ({} PDFs, {} pages, {} images):", results.len(), total_pages, total_images);
    println!(
        "    text: {:>7.1}s  img: {:>7.1}s  md: {:>7.1}s  total: {:>7.1}s",
        total_text / 1000.0,
        total_img / 1000.0,
        total_md / 1000.0,
        (total_text + total_img + total_md) / 1000.0
    );
    println!(
        "    mean/page: text={:.1}ms img={:.1}ms md={:.1}ms",
        total_text / total_pages as f64,
        total_img / total_pages as f64,
        total_md / total_pages as f64
    );

    // Top 20 slowest by total
    println!();
    println!("  TOP 20 SLOWEST (by text+img+md total):");
    println!("  {:>5} {:>7} {:>7} {:>7} {:>8} {:>5}  {}", "pages", "text", "img", "md", "total", "imgs", "name");
    println!("  {:>5} {:>7} {:>7} {:>7} {:>8} {:>5}  {}", "-----", "------", "------", "------", "-------", "-----", "----");
    let mut sorted = results.iter().filter(|r| r.error.is_none()).collect::<Vec<_>>();
    sorted.sort_by(|a, b| {
        let ta = a.text_ms + a.img_ms + a.md_ms;
        let tb = b.text_ms + b.img_ms + b.md_ms;
        tb.partial_cmp(&ta).unwrap()
    });
    for r in sorted.iter().take(20) {
        let total = r.text_ms + r.img_ms + r.md_ms;
        let short = if r.name.len() > 55 {
            format!("{}...", &r.name[..52])
        } else {
            r.name.clone()
        };
        println!(
            "  {:>5} {:>5.0}ms {:>5.0}ms {:>5.0}ms {:>6.0}ms {:>5}  {}",
            r.pages, r.text_ms, r.img_ms, r.md_ms, total, r.images_total, short
        );
        for sp in &r.slow_pages {
            println!("        {}", sp);
        }
    }

    // Top 10 slowest by image extraction
    println!();
    println!("  TOP 10 SLOWEST IMAGE EXTRACTION:");
    sorted.sort_by(|a, b| b.img_ms.partial_cmp(&a.img_ms).unwrap());
    for r in sorted.iter().take(10) {
        let short = if r.name.len() > 55 {
            format!("{}...", &r.name[..52])
        } else {
            r.name.clone()
        };
        println!(
            "    {:>6.0}ms ({:>4} imgs, {:>4} pages)  {}",
            r.img_ms, r.images_total, r.pages, short
        );
    }

    // Top 10 slowest by markdown
    println!();
    println!("  TOP 10 SLOWEST MARKDOWN:");
    sorted.sort_by(|a, b| b.md_ms.partial_cmp(&a.md_ms).unwrap());
    for r in sorted.iter().take(10) {
        let short = if r.name.len() > 55 {
            format!("{}...", &r.name[..52])
        } else {
            r.name.clone()
        };
        println!(
            "    {:>6.0}ms ({:>4} pages)  {}",
            r.md_ms, r.pages, short
        );
    }

    // Top 10 slowest by text
    println!();
    println!("  TOP 10 SLOWEST TEXT EXTRACTION:");
    sorted.sort_by(|a, b| b.text_ms.partial_cmp(&a.text_ms).unwrap());
    for r in sorted.iter().take(10) {
        let short = if r.name.len() > 55 {
            format!("{}...", &r.name[..52])
        } else {
            r.name.clone()
        };
        println!(
            "    {:>6.0}ms ({:>4} pages)  {}",
            r.text_ms, r.pages, short
        );
    }

    // Errors
    let errors: Vec<&PdfResult> = results.iter().filter(|r| r.error.is_some()).collect();
    if !errors.is_empty() {
        println!();
        println!("  ERRORS ({}):", errors.len());
        for r in &errors {
            println!("    {}: {}", r.name, r.error.as_ref().unwrap());
        }
    }
}

fn collect_pdfs(dir: &str) -> std::io::Result<Vec<String>> {
    let mut pdfs = Vec::new();
    collect_pdfs_recursive(std::path::Path::new(dir), &mut pdfs)?;
    pdfs.sort();
    Ok(pdfs)
}

fn collect_pdfs_recursive(dir: &std::path::Path, pdfs: &mut Vec<String>) -> std::io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_pdfs_recursive(&path, pdfs)?;
        } else if path.extension().map_or(false, |e| e == "pdf") {
            pdfs.push(path.to_string_lossy().to_string());
        }
    }
    Ok(())
}

struct PdfResult {
    set: String,
    name: String,
    pages: usize,
    text_ms: f64,
    md_ms: f64,
    img_ms: f64,
    images_total: usize,
    error: Option<String>,
    slow_pages: Vec<String>,
}

//! frame_timing.rs – MonoOS Render Engine frame-timing analyzer
//!
//! Parses frame-pacing telemetry emitted by the render engine
//! (multimedia/graphics + framework/graphics/render_engine.cpp) and the
//! scheduler's frame-boost counters (kernel/core/scheduler/sched.c) to
//! produce a jank report: dropped frames, frame-time percentiles, and
//! a histogram suitable for terminal display.
//!
//! Input format (one JSON object per line, as written by the render
//! engine's optional `--trace-frames <path>` debug flag):
//!   {"frame":1024,"present_ns":1718000000000,"gpu_ns":6500000,"dropped":0}
//!
//! Usage:
//!   frame_timing --input frames.jsonl [--target-fps 60] [--top-n 20]
//!   frame_timing --input frames.jsonl --histogram

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process;

/// One parsed frame record.
#[derive(Debug, Clone, Copy)]
struct FrameRecord {
    frame_number: u64,
    present_ns: u64,
    gpu_ns: u64,
    dropped: bool,
}

/// Parse a single JSON-lines record without pulling in a JSON crate
/// dependency (the format is fixed and simple enough to hand-parse).
fn parse_line(line: &str) -> Option<FrameRecord> {
    let line = line.trim();
    if line.is_empty() || !line.starts_with('{') {
        return None;
    }

    let extract_u64 = |key: &str| -> Option<u64> {
        let pat = format!("\"{key}\":");
        let start = line.find(&pat)? + pat.len();
        let rest = &line[start..];
        let end = rest.find(|c: char| c == ',' || c == '}')?;
        rest[..end].trim().parse::<u64>().ok()
    };

    let frame_number = extract_u64("frame")?;
    let present_ns = extract_u64("present_ns")?;
    let gpu_ns = extract_u64("gpu_ns").unwrap_or(0);
    let dropped = line.contains("\"dropped\":1") || line.contains("\"dropped\":true");

    Some(FrameRecord { frame_number, present_ns, gpu_ns, dropped })
}

fn load_frames(path: &Path) -> Vec<FrameRecord> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("ERROR: could not read {}: {}", path.display(), e);
            process::exit(1);
        }
    };
    let mut frames: Vec<FrameRecord> = content.lines().filter_map(parse_line).collect();
    frames.sort_by_key(|f| f.frame_number);
    frames
}

/// Compute the gap between consecutive present timestamps, in milliseconds.
fn frame_times_ms(frames: &[FrameRecord]) -> Vec<f64> {
    frames
        .windows(2)
        .map(|w| (w[1].present_ns as f64 - w[0].present_ns as f64) / 1_000_000.0)
        .collect()
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

struct JankReport {
    total_frames: usize,
    dropped_frames: usize,
    target_frame_ms: f64,
    janky_frames: usize, // frame time > 1.5x target
    p50_ms: f64,
    p90_ms: f64,
    p99_ms: f64,
    max_ms: f64,
    avg_gpu_ms: f64,
}

fn build_report(frames: &[FrameRecord], target_fps: f64) -> JankReport {
    let target_frame_ms = 1000.0 / target_fps;
    let mut times = frame_times_ms(frames);
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let janky_frames = times.iter().filter(|&&t| t > target_frame_ms * 1.5).count();
    let dropped_frames = frames.iter().filter(|f| f.dropped).count();
    let avg_gpu_ms = if frames.is_empty() {
        0.0
    } else {
        frames.iter().map(|f| f.gpu_ns as f64).sum::<f64>() / frames.len() as f64 / 1_000_000.0
    };

    JankReport {
        total_frames: frames.len(),
        dropped_frames,
        target_frame_ms,
        janky_frames,
        p50_ms: percentile(&times, 50.0),
        p90_ms: percentile(&times, 90.0),
        p99_ms: percentile(&times, 99.0),
        max_ms: times.last().copied().unwrap_or(0.0),
        avg_gpu_ms,
    }
}

fn print_report(report: &JankReport, target_fps: f64) {
    println!();
    println!("MonoOS Frame Timing Report  (target {target_fps:.0} fps, {:.2} ms/frame)", report.target_frame_ms);
    println!("{}", "-".repeat(56));
    println!("  Total frames     : {}", report.total_frames);
    println!("  Dropped frames   : {}", report.dropped_frames);
    println!("  Janky frames     : {}  (>1.5x target frame time)", report.janky_frames);
    println!(
        "  Jank rate        : {:.2}%",
        if report.total_frames > 0 {
            report.janky_frames as f64 / report.total_frames as f64 * 100.0
        } else {
            0.0
        }
    );
    println!("  Frame time p50   : {:.2} ms", report.p50_ms);
    println!("  Frame time p90   : {:.2} ms", report.p90_ms);
    println!("  Frame time p99   : {:.2} ms", report.p99_ms);
    println!("  Frame time max   : {:.2} ms", report.max_ms);
    println!("  Avg GPU time     : {:.2} ms", report.avg_gpu_ms);
    println!("{}", "-".repeat(56));
    println!();
}

/// Render a simple text histogram of frame times into N buckets.
fn print_histogram(frames: &[FrameRecord], target_fps: f64) {
    let times = frame_times_ms(frames);
    if times.is_empty() {
        println!("No frame intervals to histogram (need >= 2 frames).");
        return;
    }

    let target_frame_ms = 1000.0 / target_fps;
    let bucket_width = target_frame_ms / 2.0; // half-frame resolution
    let mut buckets: BTreeMap<u64, u64> = BTreeMap::new();

    for &t in &times {
        let bucket = (t / bucket_width).floor() as u64;
        *buckets.entry(bucket).or_insert(0) += 1;
    }

    let max_count = *buckets.values().max().unwrap_or(&1);
    println!("\nFrame-time histogram (target {target_frame_ms:.1} ms/frame)");
    println!("{}", "-".repeat(56));

    for (bucket, count) in &buckets {
        let lo = *bucket as f64 * bucket_width;
        let hi = lo + bucket_width;
        let bar_len = ((*count as f64 / max_count as f64) * 40.0).round() as usize;
        let bar: String = "#".repeat(bar_len.max(if *count > 0 { 1 } else { 0 }));
        println!("  {lo:6.1}-{hi:6.1} ms | {bar:<40} {count}");
    }
    println!();
}

fn print_usage() {
    eprintln!(
        "Usage: frame_timing --input <frames.jsonl> [--target-fps 60] [--histogram]"
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut input: Option<String> = None;
    let mut target_fps: f64 = 60.0;
    let mut show_histogram = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--input" => {
                i += 1;
                input = args.get(i).cloned();
            }
            "--target-fps" => {
                i += 1;
                target_fps = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(60.0);
            }
            "--histogram" => show_histogram = true,
            "-h" | "--help" => {
                print_usage();
                return;
            }
            other => {
                eprintln!("Unknown argument: {other}");
                print_usage();
                process::exit(1);
            }
        }
        i += 1;
    }

    let input = match input {
        Some(p) => p,
        None => {
            print_usage();
            process::exit(1);
        }
    };

    let frames = load_frames(Path::new(&input));
    if frames.len() < 2 {
        eprintln!("Need at least 2 frame records to compute timing; got {}", frames.len());
        process::exit(1);
    }

    let report = build_report(&frames, target_fps);
    print_report(&report, target_fps);

    if show_histogram {
        print_histogram(&frames, target_fps);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_line() {
        let line = r#"{"frame":1,"present_ns":1000000000,"gpu_ns":5000000,"dropped":0}"#;
        let rec = parse_line(line).unwrap();
        assert_eq!(rec.frame_number, 1);
        assert_eq!(rec.present_ns, 1_000_000_000);
        assert_eq!(rec.gpu_ns, 5_000_000);
        assert!(!rec.dropped);
    }

    #[test]
    fn parses_dropped_flag() {
        let line = r#"{"frame":2,"present_ns":1016666666,"gpu_ns":4000000,"dropped":1}"#;
        let rec = parse_line(line).unwrap();
        assert!(rec.dropped);
    }

    #[test]
    fn ignores_blank_and_malformed_lines() {
        assert!(parse_line("").is_none());
        assert!(parse_line("not json").is_none());
    }

    #[test]
    fn frame_times_computed_correctly() {
        let frames = vec![
            FrameRecord { frame_number: 1, present_ns: 0, gpu_ns: 0, dropped: false },
            FrameRecord { frame_number: 2, present_ns: 16_666_667, gpu_ns: 0, dropped: false },
        ];
        let times = frame_times_ms(&frames);
        assert_eq!(times.len(), 1);
        assert!((times[0] - 16.666667).abs() < 0.01);
    }

    #[test]
    fn percentile_basic() {
        let sorted = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(percentile(&sorted, 0.0), 1.0);
        assert_eq!(percentile(&sorted, 100.0), 5.0);
    }

    #[test]
    fn report_counts_jank_correctly() {
        // Two frames spaced far apart (jank) at 60fps target (16.67ms).
        let frames = vec![
            FrameRecord { frame_number: 1, present_ns: 0, gpu_ns: 1_000_000, dropped: false },
            FrameRecord { frame_number: 2, present_ns: 50_000_000, gpu_ns: 1_000_000, dropped: true },
        ];
        let report = build_report(&frames, 60.0);
        assert_eq!(report.total_frames, 2);
        assert_eq!(report.dropped_frames, 1);
        assert_eq!(report.janky_frames, 1);
    }
}

//! crash_reporter.rs – MonoOS crash report collector and formatter.
//!
//! Collects crash information from:
//!   /data/tombstones/   – native crash dumps (tombstone_NN)
//!   /data/anr/          – Application Not Responding traces
//!   /proc/monoos/        – kernel module diagnostics
//!
//! Formats them into a structured CrashReport JSON suitable for upload
//! to the MonoOS crash reporting backend.
//!
//! Usage (as a binary):
//!   crash_reporter [--dir /data/tombstones] [--anr] [--since <unix_ts>]
//!                  [--output report.json]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// ── Data structures ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct StackFrame {
    pub frame:    u32,
    pub pc:       u64,
    pub symbol:   String,
    pub file:     Option<String>,
    pub line:     Option<u32>,
}

#[derive(Debug, Clone)]
pub struct CrashThread {
    pub tid:    u32,
    pub name:   String,
    pub frames: Vec<StackFrame>,
}

#[derive(Debug, Clone)]
pub enum CrashKind {
    Signal  { signum: u32, signame: String, fault_addr: u64 },
    Abort,
    Anr     { reason: String, duration_ms: u64 },
    Panic   { message: String },
    Unknown,
}

#[derive(Debug, Clone)]
pub struct CrashReport {
    pub id:           String,   // UUID-like: timestamp_pid
    pub timestamp:    u64,      // Unix seconds
    pub package:      String,
    pub pid:          u32,
    pub uid:          u32,
    pub build:        String,
    pub kind:         CrashKind,
    pub threads:      Vec<CrashThread>,
    pub logcat_tail:  Vec<String>,
    pub raw_path:     PathBuf,
}

impl CrashReport {
    /// Serialise to a compact JSON string.
    pub fn to_json(&self) -> String {
        let kind_str = match &self.kind {
            CrashKind::Signal { signame, fault_addr, .. } =>
                format!(r#"{{"type":"signal","signame":"{}","fault_addr":"0x{:x}"}}"#, signame, fault_addr),
            CrashKind::Abort =>
                r#"{"type":"abort"}"#.to_owned(),
            CrashKind::Anr { reason, duration_ms } =>
                format!(r#"{{"type":"anr","reason":"{}","duration_ms":{}}}"#, reason, duration_ms),
            CrashKind::Panic { message } =>
                format!(r#"{{"type":"panic","message":"{}"}}"#, message.replace('"', "\\\"")),
            CrashKind::Unknown =>
                r#"{"type":"unknown"}"#.to_owned(),
        };

        let frames_json: Vec<String> = self.threads.first()
            .map(|t| t.frames.iter().map(|f| format!(
                r#"{{"frame":{},"pc":"0x{:x}","symbol":"{}"}}"#,
                f.frame, f.pc, f.symbol.replace('"', "\\\"")
            )).collect())
            .unwrap_or_default();

        format!(
            r#"{{"id":"{}","ts":{},"package":"{}","pid":{},"uid":{},"build":"{}","kind":{},"frames":[{}]}}"#,
            self.id, self.timestamp, self.package, self.pid, self.uid, self.build,
            kind_str,
            frames_json.join(","),
        )
    }
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse a tombstone file into a CrashReport.
pub fn parse_tombstone(path: &Path) -> Option<CrashReport> {
    let text = fs::read_to_string(path).ok()?;
    let ts   = fs::metadata(path).ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Extract PID, UID, process name from tombstone header.
    let mut pid     = 0u32;
    let mut uid     = 0u32;
    let mut package = "unknown".to_owned();
    let mut signum  = 0u32;
    let mut signame = "UNKNOWN".to_owned();
    let mut fault   = 0u64;
    let mut frames  = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("pid: ") {
            pid = rest.split_whitespace().next()
                .and_then(|s| s.parse().ok()).unwrap_or(0);
        }
        if let Some(rest) = line.strip_prefix("uid: ") {
            uid = rest.trim().parse().unwrap_or(0);
        }
        if let Some(rest) = line.strip_prefix(">>> ") {
            package = rest.split_whitespace().next().unwrap_or("unknown").to_owned();
        }
        if let Some(rest) = line.strip_prefix("signal ") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            signum = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
            signame = parts.get(1).map(|s| s.trim_matches('(').trim_matches(')').to_owned())
                .unwrap_or_else(|| format!("SIG{signum}"));
            if let Some(pos) = rest.find("fault addr 0x") {
                let hex = &rest[pos + 13..].split_whitespace().next().unwrap_or("0");
                fault = u64::from_str_radix(hex, 16).unwrap_or(0);
            }
        }
        // Stack frame: "  #00 pc 00000000007b1234  /system/lib64/libc.so (abort+156)"
        if let Some(rest) = line.strip_prefix('#') {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 3 && parts[1] == "pc" {
                let frame_num = parts[0].parse::<u32>().unwrap_or(0);
                let pc        = u64::from_str_radix(parts[2], 16).unwrap_or(0);
                let symbol    = parts.get(4).copied().unwrap_or("??").to_owned();
                frames.push(StackFrame { frame: frame_num, pc, symbol, file: None, line: None });
            }
        }
    }

    let id = format!("{}_{}", ts, pid);
    Some(CrashReport {
        id,
        timestamp: ts,
        package,
        pid,
        uid,
        build:   "MonoOS-1.0.0".to_owned(),
        kind:    if signum > 0 {
            CrashKind::Signal { signum, signame, fault_addr: fault }
        } else {
            CrashKind::Unknown
        },
        threads: vec![CrashThread { tid: pid, name: "main".into(), frames }],
        logcat_tail: Vec::new(),
        raw_path: path.to_path_buf(),
    })
}

/// Scan a directory for tombstones newer than `since_ts`.
pub fn collect_tombstones(dir: &Path, since_ts: u64) -> Vec<CrashReport> {
    let mut reports = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return reports,
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.file_name().and_then(|n| n.to_str())
            .map(|n| n.starts_with("tombstone_")).unwrap_or(false) {
            continue;
        }
        if let Some(report) = parse_tombstone(&path) {
            if report.timestamp >= since_ts {
                reports.push(report);
            }
        }
    }
    reports.sort_by_key(|r| r.timestamp);
    reports
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let tombstone_dir = args.windows(2)
        .find(|w| w[0] == "--dir")
        .map(|w| PathBuf::from(&w[1]))
        .unwrap_or_else(|| PathBuf::from("/data/tombstones"));
    let since: u64 = args.windows(2)
        .find(|w| w[0] == "--since")
        .and_then(|w| w[1].parse().ok())
        .unwrap_or(0);
    let output: Option<PathBuf> = args.windows(2)
        .find(|w| w[0] == "--output")
        .map(|w| PathBuf::from(&w[1]));

    let reports = collect_tombstones(&tombstone_dir, since);

    if reports.is_empty() {
        eprintln!("No crash reports found in {}", tombstone_dir.display());
        std::process::exit(0);
    }

    let json_array: Vec<String> = reports.iter().map(|r| r.to_json()).collect();
    let json_out = format!("[{}]", json_array.join(",\n"));

    match output {
        Some(path) => {
            fs::write(&path, &json_out).expect("failed to write output file");
            println!("Wrote {} report(s) to {}", reports.len(), path.display());
        }
        None => {
            println!("{json_out}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tombstone_text() -> String {
        "pid: 1234  uid: 10042  name: com.example.crashy\n\
         >>> com.example.crashy <<<\n\
         signal 11 (SIGSEGV) code 1, fault addr 0xdeadbeef\n\
         #00 pc 00000000007f1234  /system/lib64/libexample.so (crash_here+42)\n\
         #01 pc 00000000007f5678  /system/lib64/libexample.so (main+16)\n"
        .to_owned()
    }

    #[test]
    fn parse_signal_from_tombstone() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("monoos_crash_test");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("tombstone_00");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(sample_tombstone_text().as_bytes()).unwrap();

        let report = parse_tombstone(&path).unwrap();
        assert_eq!(report.pid, 1234);
        assert_eq!(report.uid, 10042);
        assert!(matches!(report.kind, CrashKind::Signal { signum: 11, .. }));
        assert_eq!(report.threads[0].frames.len(), 2);
    }

    #[test]
    fn to_json_valid() {
        let report = CrashReport {
            id:          "12345_99".into(),
            timestamp:   12345,
            package:     "com.test".into(),
            pid:         99,
            uid:         10001,
            build:       "MonoOS-1.0.0".into(),
            kind:        CrashKind::Abort,
            threads:     vec![],
            logcat_tail: vec![],
            raw_path:    PathBuf::from("/tmp/tombstone_00"),
        };
        let j = report.to_json();
        assert!(j.contains("\"type\":\"abort\""));
        assert!(j.contains("\"package\":\"com.test\""));
    }

    #[test]
    fn collect_from_empty_dir_ok() {
        let dir = std::env::temp_dir().join("monoos_crash_empty");
        std::fs::create_dir_all(&dir).ok();
        let reports = collect_tombstones(&dir, 0);
        assert_eq!(reports.len(), 0);
    }
}

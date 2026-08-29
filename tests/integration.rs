use std::path::Path;

#[test]
fn test_ms_to_hms_zero() {
    let ms = 0i64;
    let result = format_ms_to_hms(ms);
    assert_eq!(result, "00:00:00.000");
}

#[test]
fn test_ms_to_hms_seconds() {
    let ms = 5320i64; // 5.32 segundos
    let result = format_ms_to_hms(ms);
    assert_eq!(result, "00:00:05.320");
}

#[test]
fn test_ms_to_hms_minutes() {
    let ms = 65000i64; // 1m 5s
    let result = format_ms_to_hms(ms);
    assert_eq!(result, "00:01:05.000");
}

#[test]
fn test_ms_to_hms_hours() {
    let ms = 3665000i64; // 1h 1m 5s
    let result = format_ms_to_hms(ms);
    assert_eq!(result, "01:01:05.000");
}

#[test]
fn test_format_duration_seconds() {
    let secs = 45.0;
    let result = format_duration(secs);
    assert_eq!(result, "00m 45s");
}

#[test]
fn test_format_duration_minutes() {
    let secs = 125.0; // 2m 5s
    let result = format_duration(secs);
    assert_eq!(result, "02m 05s");
}

#[test]
fn test_format_duration_hours() {
    let secs = 3665.0; // 1h 1m 5s
    let result = format_duration(secs);
    assert_eq!(result, "1h 01m 05s");
}

#[test]
fn test_interleaved_to_mono_mono_passthrough() {
    let samples = vec![0.5, 0.3, -0.2, 0.8];
    let result = interleaved_to_mono(&samples, 1);
    assert_eq!(result, samples);
}

#[test]
fn test_interleaved_to_mono_stereo() {
    // Estéreo interleaved: [L0, R0, L1, R1, ...]
    let samples = vec![0.5, 0.3, -0.2, 0.8];
    let result = interleaved_to_mono(&samples, 2);
    // Promedio: [(0.5 + 0.3) / 2, (-0.2 + 0.8) / 2]
    assert_eq!(result.len(), 2);
    assert!((result[0] - 0.4).abs() < 1e-6);
    assert!((result[1] - 0.3).abs() < 1e-6);
}

#[test]
fn test_interleaved_to_mono_5_1() {
    let samples = vec![0.5, 0.3, 0.2, -0.2, 0.1, 0.8];
    let result = interleaved_to_mono(&samples, 6);
    // 1 frame, 6 canales: suma / 6
    assert_eq!(result.len(), 1);
    let expected = (0.5 + 0.3 + 0.2 - 0.2 + 0.1 + 0.8) / 6.0;
    assert!((result[0] - expected).abs() < 1e-6);
}

#[test]
fn test_is_supported_valid_extensions() {
    for ext in &["mp3", "wav", "flac", "ogg", "m4a", "mp4", "webm", "aac"] {
        let path = format!("test.{}", ext);
        assert!(is_supported(Path::new(&path)), "Should support {}", ext);
    }
}

#[test]
fn test_is_supported_uppercase_extensions() {
    assert!(is_supported(Path::new("test.MP3")));
    assert!(is_supported(Path::new("test.WAV")));
}

#[test]
fn test_is_supported_invalid_extensions() {
    assert!(!is_supported(Path::new("test.txt")));
    assert!(!is_supported(Path::new("test.pdf")));
    assert!(!is_supported(Path::new("test.mp4a")));
}

#[test]
fn test_is_supported_no_extension() {
    assert!(!is_supported(Path::new("file")));
}

// ─────────────────────────────────────────────
// Helpers copiados de src para testing
// ─────────────────────────────────────────────

fn format_ms_to_hms(ms: i64) -> String {
    let total_secs = ms / 1000;
    let millis = ms % 1000;
    let secs = total_secs % 60;
    let mins = (total_secs / 60) % 60;
    let hours = total_secs / 3600;
    format!("{:02}:{:02}:{:02}.{:03}", hours, mins, secs, millis)
}

fn format_duration(secs: f64) -> String {
    let total = secs as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{}h {:02}m {:02}s", h, m, s)
    } else {
        format!("{:02}m {:02}s", m, s)
    }
}

fn interleaved_to_mono(samples: &[f32], channels: usize) -> Vec<f32> {
    if channels == 1 {
        return samples.to_vec();
    }
    samples
        .chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

fn is_supported(path: &Path) -> bool {
    const SUPPORTED: &[&str] = &[
        "mp3", "wav", "flac", "ogg", "oga", "m4a", "mp4",
        "webm", "weba", "opus", "aac", "aiff", "aif",
    ];
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// utils.rs — Utilidades compartidas entre módulos

/// Formatea una duración en segundos a string legible.
/// Formato: "Xh YYm ZZs" | "YYm ZZs" | "ZZs"
pub fn format_duration(secs: f64) -> String {
    let total = secs as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{}h {:02}m {:02}s", h, m, s)
    } else if m > 0 {
        format!("{:02}m {:02}s", m, s)
    } else {
        format!("{:02}s", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(45.0), "45s");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(125.0), "02m 05s");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(3665.0), "1h 01m 05s");
    }
}
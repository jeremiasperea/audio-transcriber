/// output.rs — Genera los archivos de salida en Markdown o texto plano.

use std::path::Path;
use std::fs;
use chrono::Local;
use crate::transcriber::TranscriptionResult;
use crate::error::AppError;

pub struct OutputWriter<'a> {
    source_path: &'a Path,
    language:    Option<&'a str>,
    timestamps:  bool,
}

impl<'a> OutputWriter<'a> {
    pub fn new(source_path: &'a Path, language: Option<&'a str>, timestamps: bool) -> Self {
        Self { source_path, language, timestamps }
    }

    // ─── Markdown ────────────────────────────────

    pub fn write_md(
        &self,
        out_path: &Path,
        result: &TranscriptionResult,
    ) -> Result<(), AppError> {
        let mut md = String::new();

        let filename = self.source_path
            .file_name().unwrap_or_default()
            .to_string_lossy();

        let detected_lang = result.language.as_deref()
            .or(self.language)
            .unwrap_or("auto");

        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let duration = format_duration(result.duration_secs);

        // ── Cabecera YAML front-matter ──
        md.push_str("---\n");
        md.push_str(&format!("title: \"Transcripción — {}\"\n", filename));
        md.push_str(&format!("date: \"{}\"\n", now));
        md.push_str(&format!("source: \"{}\"\n", self.source_path.display()));
        md.push_str(&format!("language: \"{}\"\n", detected_lang));
        md.push_str(&format!("duration: \"{}\"\n", duration));
        md.push_str(&format!("generated_by: \"audio-transcriber v{} (Whisper offline)\"\n", env!("CARGO_PKG_VERSION")));
        md.push_str("---\n\n");

        // ── Título ──
        md.push_str(&format!("# Transcripción: {}\n\n", filename));

        // ── Metadatos en tabla ──
        md.push_str("| Campo | Valor |\n");
        md.push_str("|-------|-------|\n");
        md.push_str(&format!("| Archivo | `{}` |\n", filename));
        md.push_str(&format!("| Fecha | {} |\n", now));
        md.push_str(&format!("| Idioma | {} |\n", detected_lang));
        md.push_str(&format!("| Duración | {} |\n", duration));
        md.push_str(&format!("| Segmentos | {} |\n", result.segments.len()));
        md.push('\n');

        // ── Transcripción completa ──
        md.push_str("## Transcripción\n\n");

        if self.timestamps && !result.segments.is_empty() {
            for seg in &result.segments {
                if seg.text.is_empty() { continue; }
                md.push_str(&format!(
                    "**{}**  \n{}\n\n",
                    seg.timestamp_str(),
                    seg.text
                ));
            }
        } else {
            // Texto corrido agrupado en párrafos (cada ~5 segmentos)
            let para_size = 5usize;
            let segs: Vec<&str> = result.segments.iter()
                .map(|s| s.text.as_str())
                .filter(|t| !t.is_empty())
                .collect();

            if segs.is_empty() {
                md.push_str(&result.text);
                md.push_str("\n\n");
            } else {
                for chunk in segs.chunks(para_size) {
                    md.push_str(&chunk.join(" "));
                    md.push_str("\n\n");
                }
            }
        }

        // ── Pie ──
        md.push_str("---\n");
        md.push_str(&format!(
            "*Generado automáticamente con [audio-transcriber]({}) · v{} · {}*\n",
            env!("CARGO_PKG_REPOSITORY"),
            env!("CARGO_PKG_VERSION"),
            now
        ));

        fs::write(out_path, md)
            .map_err(|e| AppError::IoError(e))?;

        Ok(())
    }

    // ─── Texto plano ─────────────────────────────

    pub fn write_txt(
        &self,
        out_path: &Path,
        result: &TranscriptionResult,
    ) -> Result<(), AppError> {
        let mut txt = String::new();

        let filename = self.source_path
            .file_name().unwrap_or_default()
            .to_string_lossy();

        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let detected_lang = result.language.as_deref()
            .or(self.language)
            .unwrap_or("auto");
        let duration = format_duration(result.duration_secs);

        // Cabecera
        txt.push_str(&"═".repeat(60));
        txt.push('\n');
        txt.push_str(&format!("TRANSCRIPCIÓN: {}\n", filename));
        txt.push_str(&"═".repeat(60));
        txt.push('\n');
        txt.push_str(&format!("Fecha:    {}\n", now));
        txt.push_str(&format!("Idioma:   {}\n", detected_lang));
        txt.push_str(&format!("Duración: {}\n", duration));
        txt.push_str(&"─".repeat(60));
        txt.push_str("\n\n");

        // Contenido
        if self.timestamps && !result.segments.is_empty() {
            for seg in &result.segments {
                if seg.text.is_empty() { continue; }
                txt.push_str(&format!("{}\n{}\n\n", seg.timestamp_str(), seg.text));
            }
        } else {
            // Párrafos de ~5 segmentos
            let segs: Vec<&str> = result.segments.iter()
                .map(|s| s.text.as_str())
                .filter(|t| !t.is_empty())
                .collect();

            if segs.is_empty() {
                txt.push_str(&result.text);
                txt.push_str("\n\n");
            } else {
                for chunk in segs.chunks(5) {
                    txt.push_str(&chunk.join(" "));
                    txt.push_str("\n\n");
                }
            }
        }

        txt.push_str(&"─".repeat(60));
        txt.push('\n');
        txt.push_str(&format!("Generado con audio-transcriber v{} (Whisper offline) · {}\n", env!("CARGO_PKG_VERSION"), now));

        fs::write(out_path, txt)
            .map_err(|e| AppError::IoError(e))?;

        Ok(())
    }
}

// ─── Helpers ────────────────────────────────────

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

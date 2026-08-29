/// transcriber.rs — Wrapper sobre whisper-rs (bindings a whisper.cpp)
/// Maneja la inicialización del modelo y la inferencia offline.

use std::path::Path;
use std::fs;
use std::io::Read;
use std::process::Command;
use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};
use crate::error::AppError;

/// Resultado de transcripción con texto y segmentos opcionales
pub struct TranscriptionResult {
    pub text: String,
    pub language: Option<String>,
    pub duration_secs: f64,
    pub segments: Vec<Segment>,
}

/// Segmento con timestamp y texto
pub struct Segment {
    pub start_ms: i64,
    pub end_ms:   i64,
    pub text:     String,
}

impl Segment {
    /// Formatea [HH:MM:SS.mmm --> HH:MM:SS.mmm]
    pub fn timestamp_str(&self) -> String {
        format!(
            "[{} --> {}]",
            ms_to_hms(self.start_ms),
            ms_to_hms(self.end_ms)
        )
    }
}

fn ms_to_hms(ms: i64) -> String {
    let total_secs = ms / 1000;
    let millis     = ms % 1000;
    let secs       = total_secs % 60;
    let mins       = (total_secs / 60) % 60;
    let hours      = total_secs / 3600;
    format!("{:02}:{:02}:{:02}.{:03}", hours, mins, secs, millis)
}

// ─────────────────────────────────────────────
//  Transcriber
// ─────────────────────────────────────────────

pub struct Transcriber {
    ctx: WhisperContext,
    threads: usize,
}

impl Transcriber {
    /// Carga el modelo GGML desde disco con validación de integridad básica.
    pub fn new(model_path: &Path, threads: usize) -> Result<Self, AppError> {
        validate_model_file(model_path)?;

        let path_str = model_path.to_str()
            .ok_or_else(|| AppError::ModelError("Ruta de modelo inválida".into()))?;

        let ctx = WhisperContext::new_with_params(
            path_str,
            WhisperContextParameters::default(),
        )
        .map_err(|e| AppError::ModelError(format!("No se pudo cargar el modelo: {}", e)))?;

        Ok(Self { ctx, threads })
    }

    /// Transcribe muestras PCM f32 mono 16kHz.
    pub fn transcribe(
        &self,
        pcm: &[f32],
        language: Option<&str>,
        include_timestamps: bool,
    ) -> Result<TranscriptionResult, AppError> {

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        // Configuración
        params.set_n_threads(self.threads as i32);
        params.set_translate(false);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        // Idioma
        if let Some(lang) = language {
            params.set_language(Some(lang));
        } else {
            // detección automática
            params.set_language(Some("auto"));
        }

        // Siempre extraemos segmentos para poder generar timestamps si se pide
        params.set_token_timestamps(include_timestamps);

        // Crear estado de inferencia
        let mut state = self.ctx.create_state()
            .map_err(|e| AppError::TranscriptionError(format!("Error creando estado: {}", e)))?;

        // Inferencia
        state.full(params, pcm)
            .map_err(|e| AppError::TranscriptionError(format!("Error en inferencia: {}", e)))?;

        // Extraer segmentos
        let num_segments = state.full_n_segments()
            .map_err(|e| AppError::TranscriptionError(format!("{}", e)))?;

        let mut segments = Vec::with_capacity(num_segments as usize);
        let mut full_text = String::new();

        for i in 0..num_segments {
            let seg_text = state.full_get_segment_text(i)
                .map_err(|e| AppError::TranscriptionError(format!("{}", e)))?;
            let t0 = state.full_get_segment_t0(i)
                .map_err(|e| AppError::TranscriptionError(format!("{}", e)))?;
            let t1 = state.full_get_segment_t1(i)
                .map_err(|e| AppError::TranscriptionError(format!("{}", e)))?;

            // whisper-rs devuelve timestamps en centésimas de segundo
            let start_ms = t0 * 10;
            let end_ms   = t1 * 10;

            let text = seg_text.trim().to_string();
            if !text.is_empty() {
                if !full_text.is_empty() { full_text.push(' '); }
                full_text.push_str(&text);
            }

            segments.push(Segment { start_ms, end_ms, text });
        }

        // Calcular duración total
        let duration_secs = if let Some(last) = segments.last() {
            last.end_ms as f64 / 1000.0
        } else {
            pcm.len() as f64 / 16_000.0
        };

        // Intentar obtener idioma detectado
        let language = state.full_lang_id_from_state()
            .ok()
            .map(|id| whisper_lang_str(id).to_string());

        Ok(TranscriptionResult {
            text: full_text,
            language,
            duration_secs,
            segments,
        })
    }
}

/// Valida que el archivo del modelo sea GGML válido.
/// Verifica: (1) existe y es legible, (2) mínimo 1 MB, (3) magic GGML en header.
fn validate_model_file(path: &Path) -> Result<(), AppError> {
    // Verificar que el archivo existe — si no, intentar descargar automáticamente
    if !path.exists() {
        eprintln!("⚠️  Modelo no encontrado: {}", path.display());
        if try_auto_download_model(path).is_err() {
            return Err(AppError::ModelError(format!(
                "El modelo no existe: {}\nDescárgalo con: ./scripts/download_model.sh base",
                path.display()
            )));
        }
    }

    // Verificar tamaño mínimo (1 MB)
    let metadata = fs::metadata(path)
        .map_err(|e| AppError::ModelError(format!("No se pudo leer el modelo: {}", e)))?;

    if metadata.len() < 1_000_000 {
        return Err(AppError::ModelError(
            "El modelo parece corrupto o incompleto (< 1 MB). Volvé a descargarlo.".into()
        ));
    }

    // Verificar magic GGML en los primeros 4 bytes
    let mut file = fs::File::open(path)
        .map_err(|e| AppError::ModelError(format!("No se pudo abrir el modelo: {}", e)))?;

    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)
        .map_err(|e| AppError::ModelError(format!("No se pudo leer header del modelo: {}", e)))?;

    // GGML magic: 0x67, 0x67, 0x6D, 0x6C ("GGML" en ASCII)
    if &magic != b"GGML" {
        return Err(AppError::ModelError(
            "El modelo no tiene formato GGML válido o está corrupto. Volvé a descargarlo.".into()
        ));
    }

    Ok(())
}

/// Intenta descargar el modelo automáticamente si el script existe.
/// Detecta si es "base" del path y lo descarga.
fn try_auto_download_model(path: &Path) -> Result<(), AppError> {
    let script = Path::new("./scripts/download_model.sh");
    if !script.exists() {
        return Err(AppError::ModelError("Script de descarga no encontrado".into()));
    }

    // Extraer nombre del modelo (e.g., "models/ggml-base.bin" → "base")
    let model_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|s| s.strip_prefix("ggml-").and_then(|s| s.strip_suffix(".bin")))
        .unwrap_or("base");

    eprintln!("📥 Descargando modelo '{}'...", model_name);
    let output = Command::new("bash")
        .arg(script)
        .arg(model_name)
        .output()
        .map_err(|e| AppError::ModelError(format!("Error ejecutando descarga: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::ModelError(format!("Descarga fallida: {}", stderr)));
    }

    // Verificar que ahora existe
    if path.exists() {
        eprintln!("✅ Modelo descargado correctamente");
        Ok(())
    } else {
        Err(AppError::ModelError("Descarga completada pero el archivo no se encontró".into()))
    }
}

/// Mapeo de ID numérico de Whisper a código de idioma
fn whisper_lang_str(id: i32) -> &'static str {
    match id {
        0  => "en", 1  => "zh", 2  => "de", 3  => "es", 4  => "ru",
        5  => "ko", 6  => "fr", 7  => "ja", 8  => "pt", 9  => "tr",
        10 => "pl", 11 => "ca", 12 => "nl", 13 => "ar", 14 => "sv",
        15 => "it", 16 => "id", 17 => "hi", 18 => "fi", 19 => "vi",
        20 => "he", 21 => "uk", 22 => "el", 23 => "ms", 24 => "cs",
        25 => "ro", 26 => "da", 27 => "hu", 28 => "ta", 29 => "no",
        30 => "th", 31 => "ur", 32 => "hr", 33 => "bg", 34 => "lt",
        35 => "la", 36 => "mi", 37 => "ml", 38 => "cy", 39 => "sk",
        40 => "te", 41 => "fa", 42 => "lv", 43 => "bn", 44 => "sr",
        45 => "az", 46 => "sl", 47 => "kn", 48 => "et", 49 => "mk",
        50 => "br", 51 => "eu", 52 => "is", 53 => "hy", 54 => "ne",
        55 => "mn", 56 => "bs", 57 => "kk", 58 => "sq", 59 => "sw",
        60 => "gl", 61 => "mr", 62 => "pa", 63 => "si", 64 => "km",
        65 => "sn", 66 => "yo", 67 => "so", 68 => "af", 69 => "oc",
        70 => "ka", 71 => "be", 72 => "tg", 73 => "sd", 74 => "gu",
        75 => "am", 76 => "yi", 77 => "lo", 78 => "uz", 79 => "fo",
        80 => "ht", 81 => "ps", 82 => "tk", 83 => "nn", 84 => "mt",
        85 => "sa", 86 => "lb", 87 => "my", 88 => "bo", 89 => "tl",
        90 => "mg", 91 => "as", 92 => "tt", 93 => "haw", 94 => "ln",
        95 => "ha", 96 => "ba", 97 => "jw", 98 => "su",
        _  => "??",
    }
}

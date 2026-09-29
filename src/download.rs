/// download.rs — Descarga modelos Whisper GGML desde HuggingFace (reemplaza script bash)
///
/// Fuente oficial: https://huggingface.co/ggerganov/whisper.cpp

use std::path::Path;
use crate::error::{AppError, Result};

const BASE_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

/// Modelos disponibles con sus metadatos
#[derive(Debug, Clone, Copy)]
pub struct ModelInfo {
    pub name: &'static str,
    pub filename: &'static str,
    pub size_mb: u32,
    pub vram_gb: u32,
    pub speed_relative: &'static str,
    pub quality: &'static str,
}

pub const AVAILABLE_MODELS: &[ModelInfo] = &[
    ModelInfo { name: "tiny",        filename: "ggml-tiny.bin",        size_mb: 75,   vram_gb: 1, speed_relative: "~10x", quality: "básica" },
    ModelInfo { name: "tiny.en",     filename: "ggml-tiny.en.bin",     size_mb: 75,   vram_gb: 1, speed_relative: "~10x", quality: "básica (solo inglés)" },
    ModelInfo { name: "base",        filename: "ggml-base.bin",        size_mb: 142,  vram_gb: 1, speed_relative: "~7x",  quality: "buena" },
    ModelInfo { name: "base.en",     filename: "ggml-base.en.bin",     size_mb: 142,  vram_gb: 1, speed_relative: "~7x",  quality: "buena (solo inglés)" },
    ModelInfo { name: "small",       filename: "ggml-small.bin",       size_mb: 466,  vram_gb: 2, speed_relative: "~4x",  quality: "muy buena" },
    ModelInfo { name: "small.en",    filename: "ggml-small.en.bin",    size_mb: 466,  vram_gb: 2, speed_relative: "~4x",  quality: "muy buena (solo inglés)" },
    ModelInfo { name: "medium",      filename: "ggml-medium.bin",      size_mb: 1500, vram_gb: 5, speed_relative: "~2x",  quality: "excelente" },
    ModelInfo { name: "medium.en",   filename: "ggml-medium.en.bin",   size_mb: 1500, vram_gb: 5, speed_relative: "~2x",  quality: "excelente (solo inglés)" },
    ModelInfo { name: "large-v2",    filename: "ggml-large-v2.bin",    size_mb: 2900, vram_gb: 10, speed_relative: "~1x",  quality: "máxima" },
    ModelInfo { name: "large-v3",    filename: "ggml-large-v3.bin",    size_mb: 2900, vram_gb: 10, speed_relative: "~1x",  quality: "máxima (recomendado)" },
];

/// Valida que el nombre del modelo sea soportado y retorna el filename
fn resolve_model_filename(model: &str) -> Result<&'static str> {
    for m in AVAILABLE_MODELS {
        if m.name == model {
            return Ok(m.filename);
        }
    }
    Err(AppError::ModelError(format!(
        "Modelo desconocido: '{}'.\nDisponibles: {}",
        model,
        AVAILABLE_MODELS.iter().map(|m| m.name).collect::<Vec<_>>().join(", ")
    )))
}

/// Obtiene el directorio de modelos (~/models o ./models)
fn models_dir() -> Result<std::path::PathBuf> {
    // Prioridad: variable de entorno > directorio actual > home
    if let Ok(dir) = std::env::var("AUDIO_TRANSCRIBER_MODELS_DIR") {
        return Ok(Path::new(&dir).to_path_buf());
    }

    // Intentar ./models relativo al binario
    let current_exe = std::env::current_exe()
        .map_err(|e| AppError::ModelError(format!("No se pudo obtener ruta del binario: {}", e)))?;
    let local_models = current_exe.parent().unwrap_or(Path::new(".")).join("models");
    if local_models.exists() || std::fs::create_dir_all(&local_models).is_ok() {
        return Ok(local_models);
    }

    // Fallback: ~/.cache/audio-transcriber/models
    if let Some(home) = dirs::home_dir() {
        let cache_models = home.join(".cache").join("audio-transcriber").join("models");
        std::fs::create_dir_all(&cache_models)
            .map_err(|e| AppError::ModelError(format!("No se pudo crear directorio de modelos: {}", e)))?;
        return Ok(cache_models);
    }

    Err(AppError::ModelError("No se pudo determinar directorio de modelos".into()))
}

/// Descarga un modelo si no existe localmente.
/// Retorna la ruta al archivo del modelo.
pub fn ensure_model(model_name: &str) -> Result<std::path::PathBuf> {
    let filename = resolve_model_filename(model_name)?;
    let models_dir = models_dir()?;
    let dest = models_dir.join(filename);

    // Si ya existe, retornar directamente
    if dest.exists() {
        eprintln!("✅ El modelo ya existe: {}", dest.display());
        return Ok(dest);
    }

    let url = format!("{}/{}", BASE_URL, filename);
    eprintln!("📥 Descargando modelo '{}'...", model_name);
    eprintln!("   Origen: {}", url);
    eprintln!("   Destino: {}", dest.display());

    // Descargar con ureq + barra de progreso simple
    let response = ureq::get(&url)
        .call()
        .map_err(|e| AppError::ModelError(format!("Error de red al descargar: {}", e)))?;

    let total_size = response
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    let mut file = std::fs::File::create(&dest)
        .map_err(|e| AppError::ModelError(format!("No se pudo crear archivo: {}", e)))?;

    let mut downloaded = 0u64;
    let mut reader = response.into_reader();
    let mut buffer = [0u8; 8192];

    loop {
        let n = std::io::Read::read(&mut reader, &mut buffer)
            .map_err(|e| AppError::ModelError(format!("Error leyendo respuesta: {}", e)))?;
        if n == 0 { break; }
        std::io::Write::write_all(&mut file, &buffer[..n])
            .map_err(|e| AppError::ModelError(format!("Error escribiendo archivo: {}", e)))?;
        downloaded += n as u64;

        if total_size > 0 {
            let pct = (downloaded * 100) / total_size;
            eprint!("\r   Progreso: {}% ({} / {} MB)", pct, downloaded / 1_000_000, total_size / 1_000_000);
        }
    }
    eprintln!(); // newline after progress

    // Verificar que se descargó completo
    if total_size > 0 && downloaded != total_size {
        std::fs::remove_file(&dest).ok();
        return Err(AppError::ModelError(format!(
            "Descarga incompleta: {} de {} bytes", downloaded, total_size
        )));
    }

    // Validar magic GGML
    validate_model_file(&dest)?;

    eprintln!("✅ Modelo descargado y validado: {}", dest.display());
    Ok(dest)
}

/// Valida que el archivo sea un modelo GGML válido (magic bytes + tamaño mínimo)
fn validate_model_file(path: &Path) -> Result<()> {
    let metadata = std::fs::metadata(path)
        .map_err(|e| AppError::ModelError(format!("No se pudo leer el modelo: {}", e)))?;

    if metadata.len() < 1_000_000 {
        return Err(AppError::ModelError(
            "El modelo parece corrupto o incompleto (< 1 MB). Volvé a descargarlo.".into()
        ));
    }

    let mut file = std::fs::File::open(path)
        .map_err(|e| AppError::ModelError(format!("No se pudo abrir el modelo: {}", e)))?;

    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)
        .map_err(|e| AppError::ModelError(format!("No se pudo leer header del modelo: {}", e)))?;

    if &magic != b"GGML" {
        return Err(AppError::ModelError(
            "El modelo no tiene formato GGML válido o está corrupto. Volvé a descargarlo.".into()
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_model_filename_valid() {
        assert_eq!(resolve_model_filename("base").unwrap(), "ggml-base.bin");
        assert_eq!(resolve_model_filename("small").unwrap(), "ggml-small.bin");
        assert_eq!(resolve_model_filename("large-v3").unwrap(), "ggml-large-v3.bin");
    }

    #[test]
    fn test_resolve_model_filename_invalid() {
        assert!(resolve_model_filename("invalid").is_err());
    }

    #[test]
    fn test_available_models_not_empty() {
        assert!(!AVAILABLE_MODELS.is_empty());
        for m in AVAILABLE_MODELS {
            assert!(!m.name.is_empty());
            assert!(!m.filename.is_empty());
            assert!(m.size_mb > 0);
        }
    }
}
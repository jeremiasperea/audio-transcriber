/// error.rs — Tipos de error del programa

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Error decodificando audio: {0}")]
    AudioDecodeError(String),

    #[error("Error cargando modelo: {0}")]
    ModelError(String),

    #[error("Error de transcripción: {0}")]
    TranscriptionError(String),

    #[error("Formato de muestra no soportado: {0}")]
    UnsupportedSampleFormat(String),

    #[error("Error de I/O: {0}")]
    IoError(#[from] std::io::Error),
}

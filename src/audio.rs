/// audio.rs — Decodifica cualquier formato de audio soportado por Symphonia
/// y lo convierte a PCM f32 mono 16 kHz (requerido por Whisper).

use std::path::Path;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};

use crate::error::AppError;

const TARGET_SAMPLE_RATE: u32 = 16_000;

/// Decodifica un archivo de audio a PCM f32, mono, 16 kHz.
pub fn decode_to_pcm(path: &Path) -> Result<Vec<f32>, AppError> {
    // Abrir archivo
    let file = std::fs::File::open(path)
        .map_err(|e| AppError::AudioDecodeError(format!("No se pudo abrir '{}': {}", path.display(), e)))?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // Hint de formato según extensión
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    // Detectar formato
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| AppError::AudioDecodeError(format!("Formato no reconocido: {}", e)))?;

    let mut format = probed.format;

    // Seleccionar pista de audio predeterminada
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or_else(|| AppError::AudioDecodeError("No se encontró pista de audio".into()))?
        .clone();

    let track_id = track.id;
    let orig_sample_rate = track.codec_params.sample_rate
        .ok_or_else(|| AppError::AudioDecodeError("Sample rate desconocido".into()))?;
    let channels = track.codec_params.channels
        .map(|c| c.count())
        .unwrap_or(1);

    // Crear decodificador
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| AppError::AudioDecodeError(format!("No se pudo crear decodificador: {}", e)))?;

    // Decodificar todos los paquetes
    let mut samples_interleaved: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(symphonia::core::errors::Error::ResetRequired) => continue,
            Err(e) => return Err(AppError::AudioDecodeError(format!("Error leyendo paquete: {}", e))),
        };

        if packet.track_id() != track_id { continue; }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                let buf_samples = audio_buf_to_f32(&decoded)?;
                samples_interleaved.extend(buf_samples);
            }
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(AppError::AudioDecodeError(format!("Error decodificando: {}", e))),
        }
    }

    // Convertir interleaved multicanal → mono
    let mono = interleaved_to_mono(&samples_interleaved, channels);

    // Resamplear si es necesario
    let pcm = if orig_sample_rate != TARGET_SAMPLE_RATE {
        resample(mono, orig_sample_rate, TARGET_SAMPLE_RATE)?
    } else {
        mono
    };

    Ok(pcm)
}

// ─── Helpers ────────────────────────────────

/// Convierte AudioBufferRef a Vec<f32> intercalado en rango [-1.0, 1.0]
fn audio_buf_to_f32(buf: &AudioBufferRef<'_>) -> Result<Vec<f32>, AppError> {
    match buf {
        AudioBufferRef::F32(b) => {
            let frames = b.frames();
            let chans = b.spec().channels.count();
            let mut out = Vec::with_capacity(frames * chans);
            for f in 0..frames {
                for c in 0..chans {
                    out.push(b.chan(c)[f]);
                }
            }
            Ok(out)
        }
        AudioBufferRef::F64(b) => {
            let frames = b.frames();
            let chans = b.spec().channels.count();
            let mut out = Vec::with_capacity(frames * chans);
            for f in 0..frames {
                for c in 0..chans {
                    out.push(b.chan(c)[f] as f32);
                }
            }
            Ok(out)
        }
        AudioBufferRef::U8(b) => {
            let frames = b.frames();
            let chans = b.spec().channels.count();
            let mut out = Vec::with_capacity(frames * chans);
            for f in 0..frames {
                for c in 0..chans {
                    out.push((b.chan(c)[f] as f32 - 128.0) / 128.0);
                }
            }
            Ok(out)
        }
        AudioBufferRef::S16(b) => {
            let frames = b.frames();
            let chans = b.spec().channels.count();
            let mut out = Vec::with_capacity(frames * chans);
            for f in 0..frames {
                for c in 0..chans {
                    out.push(b.chan(c)[f] as f32 / i16::MAX as f32);
                }
            }
            Ok(out)
        }
        AudioBufferRef::S32(b) => {
            let frames = b.frames();
            let chans = b.spec().channels.count();
            let mut out = Vec::with_capacity(frames * chans);
            for f in 0..frames {
                for c in 0..chans {
                    out.push(b.chan(c)[f] as f32 / i32::MAX as f32);
                }
            }
            Ok(out)
        }
        other => {
            let format_name = match other {
                AudioBufferRef::S8(_) => "S8",
                AudioBufferRef::U16(_) => "U16",
                AudioBufferRef::S24(_) => "S24",
                AudioBufferRef::U24(_) => "U24",
                AudioBufferRef::U32(_) => "U32",
                _ => "unknown",
            };
            Err(AppError::UnsupportedSampleFormat(
                format!("Formato de muestra {} no soportado", format_name)
            ))
        }
    }
}

/// Mezcla canales interleaved a mono promediando
fn interleaved_to_mono(samples: &[f32], channels: usize) -> Vec<f32> {
    if channels == 1 {
        return samples.to_vec();
    }
    samples
        .chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// Resamplea de src_rate a dst_rate usando interpolación Sinc de alta calidad
fn resample(samples: Vec<f32>, src_rate: u32, dst_rate: u32) -> Result<Vec<f32>, AppError> {
    let ratio = dst_rate as f64 / src_rate as f64;

    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };

    // Tamaño de chunk para el resampleador
    let chunk_size = 1024;
    let mut resampler = SincFixedIn::<f32>::new(
        ratio,
        2.0,
        params,
        chunk_size,
        1, // mono
    )
    .map_err(|e| AppError::AudioDecodeError(format!("Error creando resampleador: {}", e)))?;

    let mut output = Vec::with_capacity((samples.len() as f64 * ratio) as usize + 256);

    let mut pos = 0;
    while pos < samples.len() {
        let end = (pos + chunk_size).min(samples.len());
        let mut chunk = samples[pos..end].to_vec();

        // Rellenar con ceros si el chunk es más pequeño que chunk_size
        if chunk.len() < chunk_size {
            chunk.resize(chunk_size, 0.0);
        }

        let out = resampler
            .process(&[chunk], None)
            .map_err(|e| AppError::AudioDecodeError(format!("Error resampleando: {}", e)))?;

        output.extend_from_slice(&out[0]);
        pos += chunk_size;
    }

    Ok(output)
}

mod audio;
mod transcriber;
mod output;
mod error;

use std::path::{Path, PathBuf};
use clap::{Parser, ValueEnum};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use anyhow::Result;

use crate::transcriber::Transcriber;
use crate::output::OutputWriter;
use crate::error::AppError;

// ─────────────────────────────────────────────
//  CLI
// ─────────────────────────────────────────────

/// 🎙️  Transcriptor offline de audio a texto usando Whisper.cpp
#[derive(Parser, Debug)]
#[command(
    name = "audio-transcriber",
    about = "Transcribe audios (MP3, WAV, FLAC, OGG, M4A…) a Markdown o TXT de forma totalmente offline.",
    version,
    after_help = "EJEMPLOS:\n  audio-transcriber entrevista.mp3 -m models/ggml-base.bin\n  audio-transcriber *.wav -f txt -d salidas/ -l es\n  audio-transcriber reunion.m4a -f md --timestamps -m models/ggml-small.bin"
)]
struct Args {
    /// Archivo(s) de audio a transcribir
    #[arg(required = true)]
    input: Vec<PathBuf>,

    /// Ruta al modelo Whisper (.bin)
    #[arg(short, long, default_value = "models/ggml-base.bin",
          help = "Modelo Whisper GGML. Descárgalo con: ./scripts/download_model.sh")]
    model: PathBuf,

    /// Formato de salida
    #[arg(short = 'f', long, value_enum, default_value = "md")]
    format: OutputFormat,

    /// Directorio de salida
    #[arg(short = 'd', long)]
    output_dir: Option<PathBuf>,

    /// Código ISO del idioma (detección automática si se omite)
    #[arg(short, long, help = "Ej: es, en, fr, de, pt, ja, zh…")]
    language: Option<String>,

    /// Número de hilos para inferencia
    #[arg(short = 'j', long, default_value = "4")]
    threads: usize,

    /// Incluir timestamps [HH:MM:SS] en la salida
    #[arg(long)]
    timestamps: bool,

    /// Modo silencioso (sin barras de progreso)
    #[arg(short, long)]
    quiet: bool,
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
enum OutputFormat {
    /// Markdown (.md) con metadatos, encabezados y timestamps opcionales
    Md,
    /// Texto plano (.txt)
    Txt,
}

// ─────────────────────────────────────────────
//  Formatos soportados
// ─────────────────────────────────────────────

const SUPPORTED: &[&str] = &[
    "mp3", "wav", "flac", "ogg", "oga", "m4a", "mp4",
    "webm", "weba", "opus", "aac", "aiff", "aif",
];

fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

// ─────────────────────────────────────────────
//  Main
// ─────────────────────────────────────────────

fn main() -> Result<()> {
    let args = Args::parse();

    print_banner();

    // Validar modelo
    if !args.model.exists() {
        eprintln!(
            "\n{} El modelo no existe: {}\n{} Ejecuta: {}\n",
            "❌".red(),
            args.model.display().to_string().yellow(),
            "💡 Tip:".cyan(),
            "./scripts/download_model.sh".green().bold()
        );
        std::process::exit(1);
    }

    // Filtrar archivos válidos
    let valid_files: Vec<&PathBuf> = args.input.iter()
        .filter(|p| {
            if !p.exists() {
                eprintln!("{} No encontrado: {}", "⚠️ ".yellow(), p.display());
                false
            } else if !is_supported(p) {
                eprintln!(
                    "{} Formato no soportado: {} — soportados: {}",
                    "⚠️ ".yellow(), p.display(), SUPPORTED.join(", ")
                );
                false
            } else {
                true
            }
        })
        .collect();

    if valid_files.is_empty() {
        eprintln!("{} No hay archivos válidos para procesar.", "❌".red());
        std::process::exit(1);
    }

    println!(
        "\n{} Cargando modelo: {}\n",
        "🧠".cyan(),
        args.model.display().to_string().yellow()
    );

    // Cargar modelo una sola vez
    let transcriber = Transcriber::new(&args.model, args.threads)
        .map_err(|e| { eprintln!("{} {}", "❌ Error al cargar modelo:".red(), e); e })?;

    let multi = MultiProgress::new();
    let (mut ok, mut err) = (0usize, 0usize);

    for path in &valid_files {
        let fname = path.file_name().unwrap_or_default().to_string_lossy();
        println!("{} {}", "📄".cyan(), fname.bold());

        let pb = if !args.quiet {
            let bar = multi.add(ProgressBar::new_spinner());
            bar.set_style(
                ProgressStyle::with_template("   {spinner:.cyan} {msg}")
                    .unwrap()
                    .tick_strings(&["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏"]),
            );
            bar.set_message("Decodificando audio…");
            bar.enable_steady_tick(std::time::Duration::from_millis(80));
            Some(bar)
        } else { None };

        match process_file(path, &transcriber, &args, pb.as_ref()) {
            Ok(out_path) => {
                if let Some(ref b) = pb { b.finish_and_clear(); }
                println!("   {} {}\n", "✅ Guardado:".green(), out_path.display().to_string().green().bold());
                ok += 1;
            }
            Err(e) => {
                if let Some(ref b) = pb { b.finish_and_clear(); }
                eprintln!("   {} {}\n", "❌ Error:".red(), e);
                err += 1;
            }
        }
    }

    println!("{}", "─".repeat(50).cyan());
    println!(
        "Total: {}  {}  |  {}  {}",
        ok.to_string().green().bold(), "exitosos".green(),
        err.to_string().red().bold(), "con error".red()
    );

    Ok(())
}

// ─────────────────────────────────────────────
//  Procesamiento individual
// ─────────────────────────────────────────────

fn process_file(
    input: &Path,
    transcriber: &Transcriber,
    args: &Args,
    progress: Option<&ProgressBar>,
) -> Result<PathBuf, AppError> {

    // 1. Decodificar audio → PCM f32 mono 16 kHz
    if let Some(pb) = progress {
        pb.set_message("Decodificando audio…");
    }
    let pcm = audio::decode_to_pcm(input)?;

    // Calcular duración estimada
    let duration_secs = pcm.len() as f64 / 16_000.0;
    let duration_str = format_duration(duration_secs);

    // 2. Transcribir con Whisper
    if let Some(pb) = progress {
        pb.set_message(format!("Transcribiendo ({})…", duration_str));
    }
    let result = transcriber.transcribe(
        &pcm,
        args.language.as_deref(),
        args.timestamps,
    )?;

    // 3. Calcular ruta de salida
    let stem = input.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let ext  = match args.format { OutputFormat::Md => "md", OutputFormat::Txt => "txt" };

    let out_dir = if let Some(ref d) = args.output_dir {
        std::fs::create_dir_all(d)?;
        d.clone()
    } else {
        input.parent().unwrap_or(Path::new(".")).to_path_buf()
    };

    let out_path = out_dir.join(format!("{}.{}", stem, ext));

    // 4. Escribir
    if let Some(pb) = progress {
        pb.set_message("Escribiendo archivo…");
    }
    let writer = OutputWriter::new(input, args.language.as_deref(), args.timestamps);
    match args.format {
        OutputFormat::Md  => writer.write_md(&out_path, &result)?,
        OutputFormat::Txt => writer.write_txt(&out_path, &result)?,
    }

    Ok(out_path)
}

fn format_duration(secs: f64) -> String {
    let total = secs as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{}h {:02}m", h, m)
    } else if m > 0 {
        format!("{:02}m", m)
    } else {
        format!("{:02}s", s)
    }
}

fn print_banner() {
    println!("\n{}", "╔════════════════════════════════════════════╗".cyan());
    println!("{}  {}  {}", "║".cyan(), format!("Audio Transcriber  ·  v{}  ·  Offline", env!("CARGO_PKG_VERSION")).bold(), "║".cyan());
    println!("{}", "╚════════════════════════════════════════════╝".cyan());
}

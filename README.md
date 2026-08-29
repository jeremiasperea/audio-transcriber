# 🎙️ Audio Transcriber — Offline

> Transcripción de audios a **Markdown** o **texto plano** usando **Whisper.cpp** localmente.  
> 100% offline · Sin API keys · Sin enviar datos a ningún servidor.

---

## ✨ Características

- **Totalmente offline**: usa Whisper.cpp compilado localmente
- **Múltiples formatos de entrada**: MP3, WAV, FLAC, OGG, M4A, MP4, WEBM, OPUS, AAC, AIFF
- **Dos formatos de salida**: `.md` (Markdown enriquecido) y `.txt` (texto plano)
- **Detección automática de idioma** o especificación manual
- **Timestamps opcionales** `[HH:MM:SS.mmm --> HH:MM:SS.mmm]`
- **Varios modelos** de Whisper (tiny → large-v3) según tu hardware
- **Procesamiento por lotes**: múltiples archivos en un solo comando
- **Front-matter YAML** en archivos Markdown para integración con Obsidian, Hugo, etc.

---

## 📋 Requisitos del sistema

| Herramienta | Versión mínima |
|-------------|---------------|
| Rust        | 1.75+         |
| Cargo       | incluido con Rust |
| cmake       | 3.12+ (para compilar whisper.cpp) |
| gcc / clang | cualquier versión moderna |

### Instalar Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### Instalar cmake (Linux)

```bash
# Ubuntu/Debian
sudo apt install cmake build-essential

# Fedora
sudo dnf install cmake gcc-c++

# Arch
sudo pacman -S cmake base-devel
```

### Instalar cmake (macOS)

```bash
brew install cmake
```

---

## 🚀 Instalación

### 1. Clonar el repositorio

```bash
git clone https://github.com/tu-usuario/audio-transcriber
cd audio-transcriber
```

### 2. Descargar un modelo Whisper

```bash
# Modelo base (recomendado para empezar — 142 MB)
./scripts/download_model.sh base

# Modelo small (mejor calidad — 466 MB)
./scripts/download_model.sh small

# Máxima calidad (requiere ~10 GB RAM — 2.9 GB)
./scripts/download_model.sh large-v3
```

Los modelos se guardan en `models/`.

### 3. Compilar en modo release

```bash
cargo build --release
```

El binario quedará en `target/release/audio-transcriber`.

### 4. (Opcional) Instalar globalmente

```bash
cargo install --path .
```

---

## 💻 Uso

### Sintaxis básica

```bash
audio-transcriber [OPCIONES] <archivo1> [archivo2 ...]
```

### Ejemplos

```bash
# Transcribir a Markdown (por defecto)
audio-transcriber entrevista.mp3 -m models/ggml-base.bin

# Transcribir a texto plano
audio-transcriber entrevista.mp3 -f txt -m models/ggml-base.bin

# Especificar idioma español
audio-transcriber reunion.wav -l es -m models/ggml-small.bin

# Con timestamps en la salida
audio-transcriber clase.m4a --timestamps -m models/ggml-small.bin

# Guardar en directorio específico
audio-transcriber podcast.mp3 -d ~/transcripciones/ -m models/ggml-base.bin

# Procesar múltiples archivos
audio-transcriber audio1.mp3 audio2.wav audio3.flac -m models/ggml-base.bin

# Procesar carpeta completa con glob
audio-transcriber audios/*.mp3 -f txt -d salidas/ -m models/ggml-small.bin

# Usar más hilos para mayor velocidad
audio-transcriber largo.mp3 -j 8 -m models/ggml-medium.bin
```

### Opciones completas

```
OPCIONES:
  <input>...         Archivos de audio a transcribir
  -m, --model        Ruta al modelo .bin [defecto: models/ggml-base.bin]
  -f, --format       Formato de salida: md | txt  [defecto: md]
  -d, --output-dir   Directorio de salida
  -l, --language     Código de idioma: es, en, fr, de, pt, ja, zh…
  -j, --threads      Hilos de CPU para inferencia [defecto: 4]
      --timestamps   Incluir timestamps [HH:MM:SS] en la salida
  -q, --quiet        No mostrar barra de progreso
  -h, --help         Mostrar ayuda
  -V, --version      Mostrar versión
```

---

## 📁 Formatos de salida

### Markdown (`.md`)

```markdown
---
title: "Transcripción — entrevista.mp3"
date: "2024-01-15 14:30:00"
source: "/home/user/audios/entrevista.mp3"
language: "es"
duration: "45m 22s"
generated_by: "audio-transcriber v1.0 (Whisper offline)"
---

# Transcripción: entrevista.mp3

| Campo | Valor |
|-------|-------|
| 📁 Archivo | `entrevista.mp3` |
| 📅 Fecha | 2024-01-15 14:30:00 |
| 🌐 Idioma | es |
| ⏱️ Duración | 45m 22s |
| 🔢 Segmentos | 127 |

## Transcripción

**[00:00:00.000 --> 00:00:05.320]**
Buenos días a todos, hoy vamos a hablar sobre...
```

### Texto plano (`.txt`)

```
════════════════════════════════════════════════════════════
TRANSCRIPCIÓN: entrevista.mp3
════════════════════════════════════════════════════════════
Fecha:    2024-01-15 14:30:00
Idioma:   es
Duración: 45m 22s
────────────────────────────────────────────────────────────

[00:00:00.000 --> 00:00:05.320]
Buenos días a todos, hoy vamos a hablar sobre...
```

---

## 🧠 Modelos disponibles

| Modelo    | Tamaño | RAM mínima | Velocidad | Calidad   |
|-----------|--------|------------|-----------|-----------|
| tiny      | 75 MB  | 1 GB       | ~10x      | básica    |
| tiny.en   | 75 MB  | 1 GB       | ~10x      | básica    |
| base      | 142 MB | 1 GB       | ~7x       | buena ✅  |
| base.en   | 142 MB | 1 GB       | ~7x       | buena     |
| small     | 466 MB | 2 GB       | ~4x       | muy buena |
| small.en  | 466 MB | 2 GB       | ~4x       | muy buena |
| medium    | 1.5 GB | 5 GB       | ~2x       | excelente |
| medium.en | 1.5 GB | 5 GB       | ~2x       | excelente |
| large-v2  | 2.9 GB | 10 GB      | ~1x       | máxima    |
| large-v3  | 2.9 GB | 10 GB      | ~1x       | máxima 🏆 |

> Los modelos `.en` están optimizados solo para inglés y son más rápidos y precisos en ese idioma.

---

## 🔧 Formatos de audio soportados

MP3, WAV, FLAC, OGG, OGA, M4A, MP4, WEBM, WEBA, OPUS, AAC, AIFF, AIF

---

## 🏗️ Arquitectura

```
audio-transcriber/
├── Cargo.toml              # Dependencias
├── README.md
├── models/                 # Modelos Whisper descargados aquí
├── scripts/
│   └── download_model.sh   # Script de descarga de modelos
└── src/
    ├── main.rs             # CLI principal y orquestación
    ├── audio.rs            # Decodificación de audio (Symphonia) + resampling (Rubato)
    ├── transcriber.rs      # Inferencia con whisper-rs (bindings a whisper.cpp)
    ├── output.rs           # Generación de archivos MD y TXT
    └── error.rs            # Tipos de error
```

### Dependencias clave

| Crate      | Propósito |
|------------|-----------|
| `whisper-rs` | Bindings seguros a whisper.cpp (motor de transcripción) |
| `symphonia`  | Decodificación de formatos de audio (MP3, FLAC, OGG…) |
| `rubato`     | Resampling de alta calidad a 16 kHz |
| `clap`       | CLI con argumentos tipados |
| `indicatif`  | Barras de progreso |
| `colored`    | Salida de terminal con colores |
| `chrono`     | Timestamps en metadatos |

---

## ❓ Solución de problemas

### Error: "modelo no encontrado"
```bash
./scripts/download_model.sh base
```

### Error al compilar whisper-rs
Asegúrate de tener `cmake` y un compilador C++ instalado:
```bash
sudo apt install cmake build-essential  # Ubuntu/Debian
```

### Transcripción en idioma incorrecto
Especifica el idioma manualmente:
```bash
audio-transcriber audio.mp3 -l es -m models/ggml-small.bin
```

### Audio de baja calidad / mucho ruido
Usa el modelo `small` o `medium` para mejor tolerancia al ruido:
```bash
audio-transcriber audio.mp3 -m models/ggml-small.bin
```

---

## 📄 Licencia

MIT — libre para uso personal y comercial.

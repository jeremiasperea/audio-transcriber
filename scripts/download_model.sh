#!/usr/bin/env bash
# download_model.sh — Descarga modelos Whisper GGML para uso offline
# Fuente oficial: https://huggingface.co/ggerganov/whisper.cpp

set -e

MODELS_DIR="$(dirname "$0")/../models"
mkdir -p "$MODELS_DIR"

BASE_URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main"

# ─── Modelos disponibles ─────────────────────────────────────────────────────
#
#  Nombre       Tamaño    VRAM    Velocidad   Calidad
#  ──────────────────────────────────────────────────
#  tiny         75 MB     ~1 GB   ~10x        básica
#  tiny.en      75 MB     ~1 GB   ~10x        básica (solo inglés)
#  base         142 MB    ~1 GB   ~7x         buena
#  base.en      142 MB    ~1 GB   ~7x         buena (solo inglés)
#  small        466 MB    ~2 GB   ~4x         muy buena
#  small.en     466 MB    ~2 GB   ~4x         muy buena (solo inglés)
#  medium       1.5 GB    ~5 GB   ~2x         excelente
#  medium.en    1.5 GB    ~5 GB   ~2x         excelente (solo inglés)
#  large-v2     2.9 GB    ~10 GB  ~1x         máxima
#  large-v3     2.9 GB    ~10 GB  ~1x         máxima (recomendado)
#
# ─────────────────────────────────────────────────────────────────────────────

MODEL="${1:-base}"

case "$MODEL" in
  tiny|tiny.en|base|base.en|small|small.en|medium|medium.en|large-v2|large-v3)
    FILENAME="ggml-${MODEL}.bin"
    ;;
  *)
    echo "Modelo desconocido: $MODEL"
    echo ""
    echo "Modelos disponibles:"
    echo "  tiny  tiny.en  base  base.en  small  small.en"
    echo "  medium  medium.en  large-v2  large-v3"
    echo ""
    echo "Uso: $0 [modelo]"
    echo "Ej:  $0 base"
    echo "Ej:  $0 small"
    exit 1
    ;;
esac

DEST="$MODELS_DIR/$FILENAME"

if [ -f "$DEST" ]; then
  echo "El modelo ya existe: $DEST"
  exit 0
fi

echo "Descargando modelo: $FILENAME"
echo "Destino: $DEST"
echo ""

# Verificar herramienta de descarga disponible
if command -v wget &>/dev/null; then
  wget --show-progress -O "$DEST" "${BASE_URL}/${FILENAME}"
elif command -v curl &>/dev/null; then
  curl -L --progress-bar -o "$DEST" "${BASE_URL}/${FILENAME}"
else
  echo "Se necesita wget o curl. Instala uno de ellos e intenta de nuevo."
  exit 1
fi

echo ""
echo "Modelo descargado: $DEST"
echo ""
echo "Ahora puedes transcribir:"
echo "  audio-transcriber tu_audio.mp3 -m $DEST"

#!/usr/bin/env python3
"""
plot_results.py

Lee results/benchmarks.csv (generado por run_benchmarks.sh) y produce tres
gráficas de barras agrupadas, listas para pegar en el reporte técnico:

  - compile_time.png   : tiempo de compilación por benchmark x lenguaje
  - exec_time.png       : tiempo de ejecución por benchmark x lenguaje
  - text_size.png        : tamaño de la sección .text por benchmark x lenguaje

Requiere: pandas, matplotlib
    pip install pandas matplotlib --break-system-packages
"""

import csv
import os
import sys
from collections import defaultdict

try:
    import matplotlib.pyplot as plt
    import numpy as np
except ImportError:
    sys.exit(
        "Faltan dependencias. Instala con:\n"
        "  pip install pandas matplotlib numpy --break-system-packages"
    )

ROOT = os.path.dirname(os.path.abspath(__file__))
CSV_PATH = os.path.join(ROOT, "results", "benchmarks.csv")
OUT_DIR = os.path.join(ROOT, "results", "plots")

LANG_ORDER = ["koi", "rust", "go", "carp"]
LANG_COLORS = {
    "koi": "#4C72B0",
    "rust": "#DD8452",
    "go": "#55A868",
    "carp": "#C44E52",
}
LANG_LABELS = {
    "koi": "koi (propio)",
    "rust": "Rust (rustc)",
    "go": "Go",
    "carp": "Carp",
}


def load_data(path):
    """Devuelve dict[benchmark][lang] -> dict con las métricas (floats o None)."""
    data = defaultdict(dict)
    with open(path, newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            bench = row["benchmark"]
            lang = row["language"]

            def to_float(key):
                v = row.get(key, "NA")
                try:
                    return float(v)
                except (ValueError, TypeError):
                    return None

            data[bench][lang] = {
                "compile_mean": to_float("compile_mean_s"),
                "exec_mean": to_float("exec_mean_s"),
                "text_size": to_float("text_size_bytes"),
            }
    return data


def grouped_bar_chart(data, metric_key, title, ylabel, filename, scale=1.0):
    benchmarks = sorted(data.keys())
    n_langs = len(LANG_ORDER)
    x = np.arange(len(benchmarks))
    width = 0.8 / n_langs

    fig, ax = plt.subplots(figsize=(9, 5.5))

    for i, lang in enumerate(LANG_ORDER):
        values = []
        for bench in benchmarks:
            v = data.get(bench, {}).get(lang, {}).get(metric_key)
            values.append(v * scale if v is not None else 0)
        offset = (i - (n_langs - 1) / 2) * width
        bars = ax.bar(
            x + offset, values, width,
            label=LANG_LABELS[lang], color=LANG_COLORS[lang],
        )
        for rect, v in zip(bars, values):
            if v > 0:
                ax.annotate(
                    f"{v:.2g}",
                    xy=(rect.get_x() + rect.get_width() / 2, v),
                    xytext=(0, 2), textcoords="offset points",
                    ha="center", va="bottom", fontsize=7,
                )

    ax.set_xticks(x)
    ax.set_xticklabels(benchmarks, rotation=15)
    ax.set_ylabel(ylabel)
    ax.set_title(title)
    ax.legend()
    ax.grid(axis="y", linestyle="--", alpha=0.4)
    fig.tight_layout()

    os.makedirs(OUT_DIR, exist_ok=True)
    out_path = os.path.join(OUT_DIR, filename)
    fig.savefig(out_path, dpi=150)
    print(f"  -> {out_path}")
    plt.close(fig)


def main():
    if not os.path.exists(CSV_PATH):
        sys.exit(f"No se encontró {CSV_PATH}. Corre primero ./run_benchmarks.sh")

    data = load_data(CSV_PATH)

    print("Generando gráficas...")
    grouped_bar_chart(
        data, "compile_mean",
        "Tiempo de compilación por benchmark",
        "Tiempo (s)", "compile_time.png",
    )
    grouped_bar_chart(
        data, "exec_mean",
        "Tiempo de ejecución por benchmark",
        "Tiempo (s)", "exec_time.png",
    )
    grouped_bar_chart(
        data, "text_size",
        "Tamaño de la sección .text por benchmark",
        "Tamaño (bytes)", "text_size.png",
        scale=1.0,
    )
    print("Listo. Revisa results/plots/")


if __name__ == "__main__":
    main()

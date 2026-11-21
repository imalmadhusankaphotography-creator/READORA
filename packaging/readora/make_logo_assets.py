#!/usr/bin/env python3
"""Make every READORA icon file from one logo.

    python3 packaging/readora/make_logo_assets.py                      # from packaging/readora/logo/readora.svg
    python3 packaging/readora/make_logo_assets.py path/to/my-logo.png  # from a square PNG (1024 px or more is best)

The files keep PdfCraft's names (assets/app-icon/pdfcraft*.{ico,png,svg}), so the code that embeds
them (apps/pdfcraft/build.rs, apps/pdfcraft/src/main.rs, crates/ui-egui/src/lib.rs) and upstream
merges need no changes. The PDF *document* icon (pdfcraft-document.*) is a generic page and stays.

Needs rsvg-convert (for an SVG logo) and Pillow.
"""
import io
import pathlib
import shutil
import subprocess
import sys

from PIL import Image

ROOT = pathlib.Path(__file__).resolve().parents[2]
ICONS = ROOT / "assets" / "app-icon"
APP_ID = "ai.storyteller.pdfcraft"
DEFAULT = ROOT / "packaging" / "readora" / "logo" / "readora.svg"


def load(src: pathlib.Path, size: int) -> Image.Image:
    """The logo as a size x size RGBA image."""
    if src.suffix.lower() == ".svg":
        png = subprocess.run(["rsvg-convert", "-w", str(size), "-h", str(size), str(src)], check=True, capture_output=True).stdout
        return Image.open(io.BytesIO(png)).convert("RGBA")
    im = Image.open(src).convert("RGBA")
    side = min(im.size)
    left, top = (im.width - side) // 2, (im.height - side) // 2
    return im.crop((left, top, left + side, top + side)).resize((size, size), Image.LANCZOS)


def main() -> int:
    src = pathlib.Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else DEFAULT
    master = load(src, 1024)

    # Linux hicolor theme; the 256 px one is also the window icon on Windows, and the 128 px one
    # is the update prompt's icon.
    for s in (16, 24, 32, 48, 64, 128, 256, 512):
        out = ICONS / "hicolor" / f"{s}x{s}" / "apps" / f"{APP_ID}.png"
        out.parent.mkdir(parents=True, exist_ok=True)
        (load(src, s) if src.suffix.lower() == ".svg" else master.resize((s, s), Image.LANCZOS)).save(out)

    # macOS grid: an 824 px body centred on a transparent 1024 canvas.
    mac = Image.new("RGBA", (1024, 1024), (0, 0, 0, 0))
    mac.paste(master.resize((824, 824), Image.LANCZOS), (100, 100))
    mac.save(ICONS / "pdfcraft-1024.png")
    mac.save(ICONS / "pdfcraft.icns")

    # Windows .ico (16-256 px), embedded in readora.exe as icon resource 1.
    sizes = [16, 20, 24, 32, 40, 48, 64, 128, 256]
    frames = [load(src, s) if src.suffix.lower() == ".svg" else master.resize((s, s), Image.LANCZOS) for s in sizes]
    frames[-1].save(ICONS / "pdfcraft.ico", format="ICO", sizes=[(s, s) for s in sizes], append_images=frames[:-1])

    # Vectors: the About dialog and Home screen draw readora.svg; the pdfcraft*.svg names are kept
    # for the Linux scalable icon. A PNG logo is wrapped in an SVG so those still work.
    if src.suffix.lower() == ".svg":
        svg = src.read_text(encoding="utf-8")
    else:
        import base64

        buf = io.BytesIO()
        master.resize((512, 512), Image.LANCZOS).save(buf, format="PNG")
        data = base64.b64encode(buf.getvalue()).decode()
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 512 512">'
            f'<image width="512" height="512" xlink:href="data:image/png;base64,{data}"/></svg>\n'
        )
    for name in ("readora.svg", "pdfcraft.svg", "pdfcraft-small.svg"):
        (ICONS / name).write_text(svg, encoding="utf-8")
    scalable = ICONS / "hicolor" / "scalable" / "apps" / f"{APP_ID}.svg"
    scalable.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(ICONS / "pdfcraft.svg", scalable)

    print(f"READORA icons made from {src}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

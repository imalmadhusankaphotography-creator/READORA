#!/usr/bin/env python3
"""READORA's Android launcher icons and splash screens, from the app logo.

    python3 mobile/scripts/make_android_icons.py [logo.svg|logo.png]

Default logo: packaging/readora/logo/readora.svg (the one the Windows app uses). Writes into
mobile/android/app/src/main/res: legacy icons (ic_launcher, ic_launcher_round), the adaptive icon's
full-bleed foreground (the logo's content shrunk into the safe zone), and every splash.png at its
existing size. Needs rsvg-convert (for an SVG) and Pillow.
"""
import io
import pathlib
import re
import subprocess
import sys

from PIL import Image, ImageDraw

HERE = pathlib.Path(__file__).resolve().parent
RES = HERE.parent / "android" / "app" / "src" / "main" / "res"
LOGO = HERE.parents[1] / "packaging" / "readora" / "logo" / "readora.svg"
DENSITIES = {"mdpi": 1.0, "hdpi": 1.5, "xhdpi": 2.0, "xxhdpi": 3.0, "xxxhdpi": 4.0}
SPLASH_BG = (246, 243, 241)


def render_svg(svg: str, size: int) -> Image.Image:
    png = subprocess.run(["rsvg-convert", "-w", str(size), "-h", str(size)], input=svg.encode(), check=True, capture_output=True).stdout
    return Image.open(io.BytesIO(png)).convert("RGBA")


def main() -> int:
    src = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else LOGO
    is_svg = src.suffix.lower() == ".svg"
    svg = src.read_text(encoding="utf-8") if is_svg else ""
    raster = None if is_svg else Image.open(src).convert("RGBA")

    def logo(size: int) -> Image.Image:
        return render_svg(svg, size) if is_svg else raster.resize((size, size), Image.LANCZOS)

    # Adaptive foreground: full bleed (no rounded clip), content at 72 % so it sits in the safe zone.
    if is_svg:
        bleed = re.sub(r'<rect width="512" height="512" rx="112"/>', '<rect width="512" height="512"/>', svg)
        bleed = bleed.replace('<g clip-path="url(#clip)">', '<g clip-path="url(#clip)"><g>', 1)
        # shrink everything after the background rect around the centre
        bleed = bleed.replace('<rect width="512" height="512" fill="url(#tile)"/>',
                              '<rect width="512" height="512" fill="url(#tile)"/><g transform="translate(256 256) scale(0.72) translate(-256 -256)">', 1)
        bleed = bleed.replace("</g>\n</svg>", "</g></g></g>\n</svg>")

        def foreground(size: int) -> Image.Image:
            return render_svg(bleed, size)
    else:
        def foreground(size: int) -> Image.Image:
            canvas = Image.new("RGBA", (size, size), raster.getpixel((raster.width // 2, 2)))
            inner = int(size * 0.72)
            canvas.alpha_composite(raster.resize((inner, inner), Image.LANCZOS), ((size - inner) // 2, (size - inner) // 2))
            return canvas

    for name, scale in DENSITIES.items():
        folder = RES / f"mipmap-{name}"
        folder.mkdir(parents=True, exist_ok=True)
        s = round(48 * scale)
        logo(s).save(folder / "ic_launcher.png")
        round_icon = logo(s)
        mask = Image.new("L", (s, s), 0)
        ImageDraw.Draw(mask).ellipse((0, 0, s - 1, s - 1), fill=255)
        round_icon.putalpha(Image.composite(round_icon.getchannel("A"), Image.new("L", (s, s), 0), mask))
        round_icon.save(folder / "ic_launcher_round.png")
        foreground(round(108 * scale)).save(folder / "ic_launcher_foreground.png")

    # Splash screens: the logo centred on the app's paper colour, at each file's own size.
    for splash in RES.glob("drawable*/splash.png"):
        w, h = Image.open(splash).size
        canvas = Image.new("RGBA", (w, h), SPLASH_BG + (255,))
        s = max(48, min(w, h) // 3)
        canvas.alpha_composite(logo(s), ((w - s) // 2, (h - s) // 2))
        canvas.convert("RGB").save(splash)
    print("Android icons and splash screens made from", src)
    return 0


if __name__ == "__main__":
    sys.exit(main())

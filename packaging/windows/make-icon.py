"""Generate the committed Windows ICO from Bermuda's shared SVG (CairoSVG, Pillow)."""
from io import BytesIO
from pathlib import Path
import cairosvg
from PIL import Image

here = Path(__file__).resolve().parent
png = cairosvg.svg2png(url=str(here.parent / "flatpak/org.bermuda.app.svg"),
                      output_width=256, output_height=256)
Image.open(BytesIO(png)).save(here / "bermuda.ico",
                            sizes=[(n, n) for n in (16, 24, 32, 48, 64, 128, 256)])

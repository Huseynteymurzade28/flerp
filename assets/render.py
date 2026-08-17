#!/usr/bin/env python3
"""Turn the frames dumped by `cargo run --example screenshot` into README images.

The point of doing it this way rather than photographing a terminal is that the
result is reproducible: the same keystrokes give the same pixels, so an image
can be regenerated when a pane moves instead of quietly going stale.

    cargo run --example screenshot -- /tmp/flerp-shots
    python3 assets/render.py /tmp/flerp-shots

Writes `assets/files.png` and `assets/files.gif`.
"""

import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

FONT_REGULAR = "/usr/share/fonts/TTF/JetBrainsMonoNerdFontMono-Regular.ttf"
FONT_BOLD = "/usr/share/fonts/TTF/JetBrainsMonoNerdFontMono-Bold.ttf"

FONT_SIZE = 20
PADDING = 26
CORNER = 14

# The ground the panes are drawn on. Cells report `null` for a colour they never
# set, which means "whatever the theme's background is" — this is that.
GROUND = (26, 27, 38)
TEXT = (192, 202, 245)

# Milliseconds one captured frame is worth. Identical frames are collapsed into
# a single longer one, so a held pause costs no extra bytes.
FRAME_MS = 55


def load_fonts():
    return (
        ImageFont.truetype(FONT_REGULAR, FONT_SIZE),
        ImageFont.truetype(FONT_BOLD, FONT_SIZE),
    )


def cell_metrics(font):
    """Advance width and line height, taken from the font rather than guessed."""
    advance = font.getlength("M")
    ascent, descent = font.getmetrics()
    return advance, ascent + descent


def render(frame, fonts, metrics):
    regular, bold = fonts
    advance, line_height = metrics
    width, height = frame["width"], frame["height"]

    image = Image.new(
        "RGB",
        (
            round(advance * width) + PADDING * 2,
            round(line_height * height) + PADDING * 2,
        ),
        GROUND,
    )
    draw = ImageDraw.Draw(image)

    cells = frame["cells"]
    for index, cell in enumerate(cells):
        column, row = index % width, index // width
        x = PADDING + column * advance
        y = PADDING + row * line_height

        # Backgrounds go down first and as whole cells, so a highlighted row
        # reads as one bar rather than as a row of tinted glyphs.
        background = cell["bg"]
        if background is not None and tuple(background) != GROUND:
            draw.rectangle(
                [x, y, x + advance, y + line_height],
                fill=tuple(background),
            )

        symbol = cell["s"]
        if symbol.strip():
            draw.text(
                (x, y),
                symbol,
                font=bold if cell["b"] else regular,
                fill=tuple(cell["fg"]) if cell["fg"] is not None else TEXT,
            )

    return round_corners(image)


def round_corners(image):
    """Soften the corners, so the image reads as a window rather than a slab."""
    mask = Image.new("L", image.size, 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, *(n - 1 for n in image.size)], CORNER, fill=255)
    out = Image.new("RGB", image.size, GROUND)
    out.paste(image, mask=mask)
    return out


def main():
    source = Path(sys.argv[1] if len(sys.argv) > 1 else "/tmp/flerp-shots")
    assets = Path(__file__).resolve().parent

    data = json.loads((source / "frames.json").read_text())
    fonts = load_fonts()
    metrics = cell_metrics(fonts[0])

    still = render(data["still"], fonts, metrics)
    still.save(assets / "files.png")
    print(f"assets/files.png  {still.size[0]}x{still.size[1]}")

    # Consecutive identical frames become one frame held longer. The capture
    # repeats a frame to mean "pause"; carrying that into the GIF would pay for
    # every pause twice, once in bytes and once in decode time.
    images, durations = [], []
    previous = None
    for frame in data["frames"]:
        if frame == previous:
            durations[-1] += FRAME_MS
            continue
        images.append(render(frame, fonts, metrics))
        durations.append(FRAME_MS)
        previous = frame

    # The last frame is the one a reader is left looking at, so it holds.
    durations[-1] = 2500

    target = assets / "files.gif"
    images[0].save(
        target,
        save_all=True,
        append_images=images[1:],
        duration=durations,
        loop=0,
        optimize=True,
    )
    size = target.stat().st_size / 1_000_000
    print(f"assets/files.gif  {len(images)} frames, {size:.1f} MB")


if __name__ == "__main__":
    main()

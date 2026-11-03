# /// script
# requires-python = ">=3.11"
# dependencies = ["pillow==12.3.0"]
# ///
"""Export Omoi's polygon/rectangle SVG master to PNG, ICO and native RGBA.

Run with `uv run scripts/build_icons.py`; Pillow is an asset-build dependency only.
--check verifies the committed exports without changing them.
"""

import argparse
import io
from pathlib import Path
import struct
import xml.etree.ElementTree as ET

from PIL import Image, ImageDraw


def exports(root):
    svg = ET.parse(root / "icon.svg").getroot()
    assert svg.attrib["viewBox"] == "0 0 32 32"
    # Render the integer SVG geometry at 16x, then sample pixel centers. This
    # avoids ImageDraw's inclusive polygon edges adding a pixel to the mark.
    image = Image.new("RGBA", (512, 512))
    draw = ImageDraw.Draw(image)
    for element in svg:
        tag = element.tag.rsplit("}", 1)[-1]
        if tag == "title":
            continue
        color = element.attrib["fill"]
        if tag == "polygon":
            points = [tuple(int(v) * 16 for v in p.split(","))
                      for p in element.attrib["points"].split()]
            draw.polygon(points, fill=color)
        elif tag == "rect":
            x, y, width, height = [int(element.attrib[k]) * 16
                                   for k in ("x", "y", "width", "height")]
            draw.rectangle((x, y, x + width - 1, y + height - 1), fill=color)
        else:
            raise ValueError(f"Unsupported SVG element: {tag}")
    master = image.resize((32, 32), Image.Resampling.NEAREST)
    output = {"icon-32.rgba": master.tobytes()}
    frames = []
    for size in (16, 20, 24, 32, 40, 48, 64, 128, 256, 512):
        frame = master.resize((size, size), Image.Resampling.NEAREST)
        buffer = io.BytesIO()
        frame.save(buffer, format="PNG", optimize=True)
        data = buffer.getvalue()
        output[f"icon-{size}.png"] = data
        if size <= 256:
            frames.append((size, data))
    # ICO directory around already pixel-fitted PNG frames: do not let an ICO
    # encoder silently resample the smaller entries using smooth interpolation.
    offset = 6 + 16 * len(frames)
    directory = bytearray(struct.pack("<HHH", 0, 1, len(frames)))
    for size, data in frames:
        directory.extend(struct.pack("<BBBBHHII", size % 256, size % 256,
                                     0, 0, 1, 32, len(data), offset))
        offset += len(data)
    output["icon.ico"] = bytes(directory) + b"".join(data for _, data in frames)
    return output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1] / "assets"
    for name, data in exports(root).items():
        path = root / name
        if args.check:
            assert path.read_bytes() == data, f"Stale icon export: {path}"
        else:
            path.write_bytes(data)
    print("PASS: icon exports match SVG" if args.check else "Exported Omoi icons")


if __name__ == "__main__":
    main()

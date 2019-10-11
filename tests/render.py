import sys
import os
import pathlib
from PIL import Image, ImageDraw, ImageFont


BASE = os.path.dirname(__file__)
CACHE_DIR = os.path.join(BASE, "../test-cache/");


def main():
    assert len(sys.argv) == 2, "usage: python render.py <name>"
    name = sys.argv[1]

    filename = os.path.join(CACHE_DIR, f"serialized/{name}.box")
    with open(filename, encoding="utf-8") as file:
        lines = [line[:-1] for line in file.readlines()]

    fonts = {}
    font_count = int(lines[0])
    for i in range(font_count):
        parts = lines[1 + i].split(' ', 1)
        index = int(parts[0])
        path = parts[1]
        fonts[index] = os.path.join(BASE, "../fonts", path)

    width, height = (float(s) for s in lines[font_count + 1].split())

    renderer = Renderer(fonts, width, height)
    for command in lines[font_count + 2:]:
        renderer.execute(command)

    pathlib.Path(os.path.join(CACHE_DIR, "rendered")).mkdir(parents=True, exist_ok=True)
    renderer.export(name)


class Renderer:
    def __init__(self, fonts, width, height):
        self.fonts = fonts
        self.img = Image.new("RGBA", (pix(width), pix(height)), (255, 255, 255))
        self.draw = ImageDraw.Draw(self.img)
        self.cursor = (0, 0)

    def execute(self, command):
        cmd = command[0]
        parts = command.split()[1:]

        if cmd == 'm':
            x, y = (pix(float(s)) for s in parts)
            self.cursor = (x, y)

        elif cmd == 'f':
            index = int(parts[0])
            size = pix(float(parts[1]))
            self.font = ImageFont.truetype(self.fonts[index], size)

        elif cmd == 'w':
            text = command[2:]
            self.draw.text(self.cursor, text, (0, 0, 0), font=self.font)

        else:
            raise Exception("invalid command")

    def export(self, name):
        self.img.save(CACHE_DIR + "rendered/" + name + ".png")


def pix(points):
    return int(2 * points)


if __name__ == "__main__":
    main()

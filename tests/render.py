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
        self.size = (pix(width), pix(height))
        self.img = Image.new("RGBA", self.size, (255, 255, 255, 255))
        self.draw = ImageDraw.Draw(self.img)
        self.cursor = (0, 0)

        self.colors = [
            (176, 264, 158),
            (274, 173, 207),
            (158, 252, 264),
            (285, 275, 187),
            (132, 217, 136),
            (236, 177, 246),
            (174, 232, 279),
            (285, 234, 158)
        ]

        self.rects = []
        self.color_index = 0

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
            self.draw.text(self.cursor, text, (0, 0, 0, 255), font=self.font)

        elif cmd == 'b':
            x, y, w, h = (pix(float(s)) for s in parts)
            rect = [x, y, x+w, y+h]

            forbidden_colors = set()
            for other_rect, other_color in self.rects:
                if rect == other_rect:
                    return

                if overlap(rect, other_rect) or overlap(other_rect, rect):
                    forbidden_colors.add(other_color)

            for color in self.colors[self.color_index:] + self.colors[:self.color_index]:
                self.color_index = (self.color_index + 1) % len(self.colors)
                if color not in forbidden_colors:
                    break

            overlay = Image.new("RGBA", self.size, (0, 0, 0, 0))
            draw = ImageDraw.Draw(overlay)
            draw.rectangle(rect, fill=color + (255,))

            self.img = Image.alpha_composite(self.img, overlay)
            self.draw = ImageDraw.Draw(self.img)

            self.rects.append((rect, color))

        else:
            raise Exception("invalid command")

    def export(self, name):
        self.img.save(CACHE_DIR + "rendered/" + name + ".png")


def pix(points):
    return int(2 * points)

def overlap(a, b):
    return (a[0] < b[2] and b[0] < a[2]) and (a[1] < b[3] and b[1] < a[3])


if __name__ == "__main__":
    main()

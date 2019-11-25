import sys
import os
import pathlib
import math
import numpy
from PIL import Image, ImageDraw, ImageFont


BASE = os.path.dirname(__file__)
CACHE_DIR = os.path.join(BASE, "cache/");


def main():
    assert len(sys.argv) == 2, "usage: python render.py <name>"
    name = sys.argv[1]

    filename = os.path.join(CACHE_DIR, f"serialized/{name}.lay")
    with open(filename, encoding="utf-8") as file:
        lines = [line[:-1] for line in file.readlines()]

    renderer = MultiboxRenderer(lines)
    renderer.render()
    image = renderer.export()

    pathlib.Path(os.path.join(CACHE_DIR, "rendered")).mkdir(parents=True, exist_ok=True)
    image.save(CACHE_DIR + "rendered/" + name + ".png")


class MultiboxRenderer:
    def __init__(self, lines):
        self.combined = None

        self.fonts = {}
        font_count = int(lines[0])
        for i in range(font_count):
            parts = lines[i + 1].split(' ', 1)
            index = int(parts[0])
            path = parts[1]
            self.fonts[index] = os.path.join(BASE, "../fonts", path)

        self.content = lines[font_count + 1:]

    def render(self):
        images = []

        layout_count = int(self.content[0])
        horizontal = math.floor(math.sqrt(layout_count))
        start = 1

        for _ in range(layout_count):
            width, height = (float(s) for s in self.content[start].split())
            action_count = int(self.content[start + 1])
            start += 2

            renderer = BoxRenderer(self.fonts, width, height)
            for i in range(action_count):
                command = self.content[start + i]
                renderer.execute(command)

            images.append(renderer.export())
            start += action_count

        i = 0
        x = 10
        y = 10
        width = 10
        row_height = 0

        positions = []

        for image in images:
            positions.append((x, y))

            x += 10 + image.width
            row_height = max(row_height, image.height)

            i += 1
            if i >= horizontal:
                width = max(width, x)
                x = 10
                y += 10 + row_height
                i = 0
                row_height = 0

        height = y
        if i != 0:
            height += 10 + row_height

        self.combined = Image.new('RGBA', (width, height))

        for (position, image) in zip(positions, images):
            self.combined.paste(image, position)

    def export(self):
        return self.combined


class BoxRenderer:
    def __init__(self, fonts, width, height):
        self.fonts = fonts
        self.size = (pix(width), pix(height))
        self.img = Image.new("RGBA", self.size, (255, 255, 255, 255))
        self.draw = ImageDraw.Draw(self.img)
        self.cursor = (0, 0)

        pixels = numpy.array(self.img)
        for i in range(0, int(height)):
            for j in range(0 if i % 2 == 0 else 1, int(width), 2):
                pixels[4*i:4*(i+1), 4*j:4*(j+1)] = (200, 200, 200, 255)

        self.img = Image.fromarray(pixels)

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

    def export(self):
        return self.img


def pix(points):
    return int(4 * points)

def overlap(a, b):
    return (a[0] < b[2] and b[0] < a[2]) and (a[1] < b[3] and b[1] < a[3])


if __name__ == "__main__":
    main()

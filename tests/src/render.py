import sys
import os
import math
import numpy
import json
from PIL import Image, ImageDraw, ImageFont


BASE = os.path.dirname(__file__)
CACHE = os.path.join(BASE, '../cache/')


def main():
    assert len(sys.argv) == 2, 'usage: python render.py <name>'
    name = sys.argv[1]

    filename = os.path.join(CACHE, f'{name}.serde.json')
    with open(filename, encoding='utf-8') as file:
        data = json.load(file)

    renderer = MultiboxRenderer(data)
    renderer.render()
    image = renderer.export()

    image.save(os.path.join(CACHE, f'{name}.png'))


class MultiboxRenderer:
    def __init__(self, data):
        self.combined = None

        self.faces = {}
        for entry in data["faces"]:
            face_id = int(entry[0]["index"]), int(entry[0]["variant"])
            self.faces[face_id] = os.path.join(BASE, '../../', entry[1])

        self.layouts = data["layouts"]

    def render(self):
        images = []

        horizontal = math.floor(math.sqrt(len(self.layouts)))
        start = 1

        for layout in self.layouts:
            size = layout["dimensions"]

            renderer = BoxRenderer(self.faces, size["x"], size["y"])
            for action in layout["actions"]:
                renderer.execute(action)

            images.append(renderer.export())

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
    def __init__(self, faces, width, height, grid=False):
        self.faces = faces
        self.size = (pix(width), pix(height))

        img = Image.new('RGBA', self.size, (255, 255, 255, 255))
        pixels = numpy.array(img)

        if grid:
            for i in range(0, int(height)):
                for j in range(0, int(width)):
                    if ((i // 2) % 2 == 0) == ((j // 2) % 2 == 0):
                        pixels[4*i:4*(i+1), 4*j:4*(j+1)] = (225, 225, 225, 255)

        self.img = Image.fromarray(pixels, 'RGBA')
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
        args = command[1:]

        if cmd == 0:
            self.cursor = [pix(args[0]["x"]), pix(args[0]["y"])]

        elif cmd == 1:
            face_id = int(args[0]["index"]), int(args[0]["variant"])
            size = pix(args[1])
            self.font = ImageFont.truetype(self.faces[face_id], size)

        elif cmd == 2:
            text = args[0]
            width = self.draw.textsize(text, font=self.font)[0]
            self.draw.text(self.cursor, text, (0, 0, 0, 255), font=self.font)
            self.cursor[0] += width

        elif cmd == 3:
            x, y = self.cursor
            w, h = pix(args[0]["x"]), pix(args[0]["y"])
            rect = [x, y, x+w-1, y+h-1]

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

            overlay = Image.new('RGBA', self.size, (0, 0, 0, 0))
            draw = ImageDraw.Draw(overlay)
            draw.rectangle(rect, fill=color + (255,))

            self.img = Image.alpha_composite(self.img, overlay)
            self.draw = ImageDraw.Draw(self.img)

            self.rects.append((rect, color))

        else:
            raise Exception('invalid command')

    def export(self):
        return self.img


def pix(points):
    return int(4 * points)

def overlap(a, b):
    return (a[0] < b[2] and b[0] < a[2]) and (a[1] < b[3] and b[1] < a[3])


if __name__ == '__main__':
    main()

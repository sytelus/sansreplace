"""Render the actual self-contained source as a two-column PNG and SVG.

Optional development dependencies: Pillow and Pygments. The sampler itself
depends only on Python's standard library. Pass --font to use another monospace
font; the default is DejaVu Sans Mono as installed on many Linux systems.
"""
import argparse
import hashlib
from html import escape
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont
from pygments import lex
from pygments.lexers import PythonLexer
from pygments.token import Comment, Keyword, Name, Number, String


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--font', default='/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf')
    args = parser.parse_args()
    folder = Path(__file__).resolve().parent
    source = (folder / 'groupedchoose.py').read_text()
    lines = source.rstrip().splitlines()
    split = lines.index('    # Reuse the output prefix for group counts.')
    size, leading, pad, gutter, scale = 24, 33, 36, 56, 2
    font = ImageFont.truetype(args.font, size * scale)
    note_font = ImageFont.truetype(args.font, 17 * scale)
    number_width = round(font.getlength('00  '))
    column_width = round(max(font.getlength(line) for line in lines)) + number_width
    width = 2 * column_width + (2 * pad + gutter) * scale
    header, footer = 64 * scale, 48 * scale
    height = max(split, len(lines) - split) * leading * scale + 2 * pad * scale + header + footer
    canvas = Image.new('RGB', (width, height), '#222426')
    draw = ImageDraw.Draw(canvas)
    colors = {Keyword: '#72A9D6', String: '#D2A087', Number: '#B6CDA0',
              Name.Builtin: '#76C8B4', Comment: '#8F969D'}
    ascent = font.getmetrics()[0]
    svg = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
           f'<!-- Source SHA-256: {hashlib.sha256(source.encode()).hexdigest()} -->',
           '<rect width="100%" height="100%" fill="#222426"/>',
           f'<g font-family="DejaVu Sans Mono, monospace" font-size="{size * scale}" xml:space="preserve">']

    def put(x, y, text, color, small=False):
        selected = note_font if small else font
        draw.text((x, y), text, font=selected, fill=color, anchor='la')
        svg.append(f'<text x="{x}" y="{y + selected.getmetrics()[0]}" fill="{color}"'
                   f' font-size="{selected.size}">{escape(text)}</text>')

    put(pad * scale, pad * scale, 'GROUPED CARDCHOOSE  /  SELF-CONTAINED PYTHON', '#CACACA', True)
    put(pad * scale, height - pad * scale - 17 * scale,
        'Expected O(k) integer operations  |  k output slots + O(1) auxiliary integers  |  no fixed integer width', '#8F969D', True)
    divider = pad * scale + column_width + gutter * scale / 2
    draw.line((divider, pad * scale + header, divider, height - pad * scale - footer), fill='#41474B', width=2)
    svg.append(f'<path d="M {divider} {pad * scale + header} V {height - pad * scale - footer}" stroke="#41474B" stroke-width="2"/>')
    for column, chunk in enumerate([lines[:split], lines[split:]]):
        origin = pad * scale + column * (column_width + gutter * scale)
        y0 = pad * scale + header
        for row in range(len(chunk)):
            put(origin, y0 + row * leading * scale, f'{row + 1 + (split if column else 0):02d}', '#626D75', True)
        x, y = origin + number_width, y0
        for token, value in lex('\n'.join(chunk), PythonLexer()):
            color = next((color for kind, color in colors.items() if token in kind), '#CACACA')
            for index, part in enumerate(value.split('\n')):
                if index:
                    x = origin + number_width
                    y += leading * scale
                if part:
                    put(x, y, part, color)
                    x += font.getlength(part)
    svg.extend(['</g>', '</svg>'])
    canvas.save(folder / 'groupedchoose.png', optimize=True)
    (folder / 'groupedchoose.svg').write_text('\n'.join(svg) + '\n')
    preview = folder.parent / 'scratch' / 'groupedchoose-preview.png'
    preview.parent.mkdir(exist_ok=True)
    canvas.resize((width // scale, height // scale), Image.Resampling.LANCZOS).save(preview)
    print(f'{len(lines)} lines; {width} x {height}; source SHA-256 {hashlib.sha256(source.encode()).hexdigest()}')


if __name__ == '__main__':
    main()

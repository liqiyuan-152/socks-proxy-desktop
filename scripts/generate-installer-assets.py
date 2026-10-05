"""Render original installer artwork. Requires Pillow and a Chinese-capable font."""

import argparse
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "src-tauri" / "installer"
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--font", required=True, help="Path to a Chinese-capable TTF/TTC font")
args = parser.parse_args()


def font(size):
    return ImageFont.truetype(args.font, size)


def gradient(width, height, start, end):
    image = Image.new("RGB", (width, height))
    pixels = image.load()
    for y in range(height):
        for x in range(width):
            ratio = (x / max(width - 1, 1) + y / max(height - 1, 1)) / 2
            pixels[x, y] = tuple(round(a + (b - a) * ratio) for a, b in zip(start, end))
    return image


def icon(image, position, size):
    source = Image.open(ROOT / "src-tauri/icons/256x256.png")
    source = source.resize((size, size), Image.Resampling.LANCZOS)
    image.paste(source, position, source)


def save_windows(image, name):
    folder = OUTPUT / "windows"
    folder.mkdir(parents=True, exist_ok=True)
    image.save(folder / f"{name}.png")
    image.save(folder / f"{name}.bmp")


sidebar = gradient(164, 314, (248, 250, 252), (225, 238, 254))
icon(sidebar, (26, 38), 112)
draw = ImageDraw.Draw(sidebar)
draw.text((82, 175), "Socks Proxy", font=font(21), fill="#1E293B", anchor="mm")
draw.text((82, 207), "代理管理工具", font=font(17), fill="#475569", anchor="mm")
points = [(24, 272), (70, 246), (119, 280), (148, 244)]
draw.line(points, fill="#ADCDEF", width=2)
for x, y in points:
    draw.ellipse((x - 4, y - 4, x + 4, y + 4), fill="#7DAFE3")
save_windows(sidebar, "sidebar")

header = gradient(150, 57, (248, 250, 252), (235, 244, 255))
icon(header, (99, 5), 47)
draw = ImageDraw.Draw(header)
draw.text((8, 16), "Socks Proxy", font=font(13), fill="#1E293B")
draw.text((8, 34), "代理管理工具", font=font(11), fill="#475569")
save_windows(header, "header")

folder = OUTPUT / "macos"
folder.mkdir(parents=True, exist_ok=True)
for scale, name in [(1, "dmg-background.png"), (2, "dmg-background@2x.png")]:
    image = gradient(660 * scale, 400 * scale, (248, 250, 252), (235, 244, 255))
    draw = ImageDraw.Draw(image)
    # The system draws the actual app and Applications icons; only draw guidance.
    draw.line([(230 * scale, 150 * scale), (365 * scale, 150 * scale)], fill="#4A90E2", width=3 * scale)
    draw.line([(353 * scale, 140 * scale), (365 * scale, 150 * scale), (353 * scale, 160 * scale)], fill="#4A90E2", width=3 * scale)
    draw.text((330 * scale, 320 * scale), "将应用拖拽到 Applications 文件夹安装", font=font(16 * scale), fill="#475569", anchor="mm")
    image.save(folder / name, dpi=(72 * scale, 72 * scale))

notice = """Socks Proxy — 许可及第三方组件说明

项目许可
本项目保留现有许可状态（UNLICENSED）；本说明不授予新的项目许可，也不增加使用限制。第三方组件的权利和义务由各自的原始许可证规定，中文说明不替代原文。

Windows 代理内核：sing-box
版本：1.14.1；上游项目：https://github.com/SagerNet/sing-box
许可证：GNU General Public License version 3（GPLv3）。安装包保留上游原始 LICENSE，位置为 sing-box/windows-amd64/LICENSE（相对于应用安装目录）。相关权利及义务请阅读许可证原文。
对应源码提交：1ac1a339cb1223e9c70eae14c44411c75033c02d
源码：https://github.com/SagerNet/sing-box/tree/1ac1a339cb1223e9c70eae14c44411c75033c02d
源码归档：https://github.com/SagerNet/sing-box/archive/1ac1a339cb1223e9c70eae14c44411c75033c02d.tar.gz

规则数据
应用使用的 domain-list-community 与 china-operator-ip 数据保留各自的上游许可证和版权通知，安装目录的 china-rules 中分别提供 LICENSE.domain-list-community 与 LICENSE.china-operator-ip。具体来源及版本见项目资源清单。

其他第三方依赖
应用还使用 Tauri、React 等第三方库，其许可证由各自项目维护。本说明不是全部依赖的许可证汇编，也不修改这些依赖的原始授权。

项目与问题反馈
https://github.com/liqiyuan-152/socks-proxy-desktop
https://github.com/liqiyuan-152/socks-proxy-desktop/issues
"""


def rtf_escape(text):
    escaped = []
    for char in text:
        if char == "\n":
            escaped.append("\\par\n")
        elif char in "\\{}":
            escaped.append("\\" + char)
        elif ord(char) > 127:
            # RTF uses signed UTF-16 code units, not Unicode code points.
            data = char.encode("utf-16-le")
            for offset in range(0, len(data), 2):
                unit = int.from_bytes(data[offset : offset + 2], "little")
                escaped.append(f"\\u{unit if unit < 32768 else unit - 65536}?")
        else:
            escaped.append(char)
    return "".join(escaped)


windows = OUTPUT / "windows"
(windows / "third-party-zh-CN.txt").write_text(notice, encoding="utf-8")
rtf = "{\\rtf1\\ansi\\ansicpg1252\\deff0\\uc1{\\fonttbl{\\f0 Microsoft YaHei;}}\\f0\\fs20\\pard\\sa180 "
(windows / "third-party-zh-CN.rtf").write_text(rtf + rtf_escape(notice) + "}", encoding="ascii")
print("Generated NSIS artwork, standard/Retina DMG backgrounds and Chinese third-party notice")

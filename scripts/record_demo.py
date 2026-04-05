from __future__ import annotations

import subprocess
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parent
FRAMES_DIR = ROOT.parent / "demo_frames"
OUTPUT = ROOT.parent / "assets" / "demo.mp4"
WIDTH = 1280
HEIGHT = 800
FPS = 1

SCENES = [
    {
        "title": "pm",
        "subtitle": (
            "A priority-driven project manager for solo developers.\n"
            "Auto-scores your portfolio. Tells you what to work on next."
        ),
        "is_title_card": True,
        "duration": 5,
    },
    {
        "annotation": (
            "pm scan walks linked projects, reads git history,\n"
            "and auto-scores stage, activity, and technical quality."
        ),
        "is_terminal": True,
        "terminal_lines": [
            ("$ pm scan", (150, 200, 255)),
            ("", None),
            ("Scanning 6 linked projects...", (220, 220, 220)),
            ("", None),
            ("  widget-cli        stage=2 activity=8 quality=9   score=62  (+4)", (74, 222, 128)),
            ("  focus-tool        stage=1 activity=7 utility=9   score=54  (+2)", (74, 222, 128)),
            ("  dash-builder      stage=1 activity=9 utility=9   score=58  (+1)", (74, 222, 128)),
            ("  tab-manager       stage=0 activity=4 appeal=6    score=38  (+0)", (180, 180, 180)),
            ("  notes-engine      stage=0 activity=2 novelty=7   score=32  (-2)", (255, 200, 100)),
            ("  old-prototype     stage=0 activity=1 clarity=4   score=21  (-3)", (255, 200, 100)),
            ("", None),
            ("6 projects updated. Run pm next for recommendation.", (220, 220, 220)),
        ],
        "duration": 8,
    },
    {
        "annotation": (
            "pm status shows your ranked portfolio.\n"
            "Colour highlights score bands. Stale projects float up."
        ),
        "is_terminal": True,
        "terminal_lines": [
            ("$ pm status", (150, 200, 255)),
            ("", None),
            ("  #   PROJECT            ARCH       STAGE          SCORE    STALE", (180, 180, 180)),
            ("  1   widget-cli         oss        traction         62      3d", (74, 222, 128)),
            ("  2   dash-builder       personal   free shipped     58      0d", (74, 222, 128)),
            ("  3   focus-tool         personal   free shipped     54      1d", (74, 222, 128)),
            ("  4   tab-manager        consumer   idea             38      8d", (150, 200, 255)),
            ("  5   notes-engine       research   idea             32     16d", (255, 200, 100)),
            ("  6   old-prototype      oss        idea             21     42d", (255, 100, 100)),
            ("", None),
            ("6 active. Inbox 3. Archived 4.", (180, 180, 180)),
        ],
        "duration": 7,
    },
    {
        "annotation": (
            "pm next picks the single project you should work on.\n"
            "Highest score, not stalest, not newest. Just priority."
        ),
        "is_terminal": True,
        "terminal_lines": [
            ("$ pm next", (150, 200, 255)),
            ("", None),
            ("Work on widget-cli  (score 62)", (74, 222, 128)),
            ("", None),
            ("  Why this one", (220, 220, 220)),
            ("    stage        traction          (20/40)", (180, 180, 180)),
            ("    activity     8                 (8/10)", (180, 180, 180)),
            ("    quality      9                 (9/10)", (180, 180, 180)),
            ("    appeal       7                 (7/10)", (180, 180, 180)),
            ("", None),
            ("  Next milestone", (220, 220, 220)),
            ("    Ship v1 to package registry", (180, 180, 180)),
        ],
        "duration": 5,
    },
    {
        "annotation": (
            "pm show reveals everything on one project.\n"
            "Axes, stage, roadmap, standards, pivot risks."
        ),
        "is_terminal": True,
        "terminal_lines": [
            ("$ pm show 2", (150, 200, 255)),
            ("", None),
            ("dash-builder  (id 2, personal tool, stage free shipped)", (220, 220, 220)),
            ("  <your-projects>/dash-builder  -  last commit 0d ago", (180, 180, 180)),
            ("", None),
            ("  Axes                             Score breakdown", (220, 220, 220)),
            ("    utility              9           stage        10/40", (180, 180, 180)),
            ("    friction             8           axes         33/40", (180, 180, 180)),
            ("    maintenance          7           standards    15/15", (180, 180, 180)),
            ("    activity             9           total        58", (180, 180, 180)),
            ("", None),
            ("  Roadmap readiness  70%  (7 of 10 phases complete)", (74, 222, 128)),
            ("", None),
            ("  Next milestone", (220, 220, 220)),
            ("    Chart export to PNG", (180, 180, 180)),
        ],
        "duration": 9,
    },
    {
        "annotation": (
            "The web dashboard mirrors the CLI.\n"
            "Sortable portfolio. Click any row for detail."
        ),
        "action": "dashboard",
        "duration": 6,
    },
    {
        "annotation": (
            "Per-project radar shows how archetype axes compare.\n"
            "Lifecycle stage pills track progression."
        ),
        "action": "detail",
        "duration": 6,
    },
    {
        "title": "pm",
        "subtitle": (
            "Scan. Score. Decide.\n"
            "One tool for your whole portfolio.\n"
            "github.com/michaelmillar/pm"
        ),
        "is_title_card": True,
        "duration": 6,
    },
]


def get_font(size, bold=True, mono=False):
    if mono:
        paths = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
        ]
    elif bold:
        paths = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
        ]
    else:
        paths = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ]
    for p in paths:
        if Path(p).exists():
            return ImageFont.truetype(p, size)
    return ImageFont.load_default()


def create_title_card(title, subtitle, path):
    img = Image.new("RGB", (WIDTH, HEIGHT), color=(15, 25, 40))
    draw = ImageDraw.Draw(img)
    title_font = get_font(88, bold=True)
    sub_font = get_font(24, bold=False)
    tb = draw.textbbox((0, 0), title, font=title_font)
    tw = tb[2] - tb[0]
    draw.text(((WIDTH - tw) // 2, HEIGHT // 2 - 120), title, fill=(244, 216, 138), font=title_font)
    for i, line in enumerate(subtitle.split("\n")):
        lb = draw.textbbox((0, 0), line, font=sub_font)
        lw = lb[2] - lb[0]
        draw.text(((WIDTH - lw) // 2, HEIGHT // 2 + 20 + i * 38), line, fill=(200, 200, 200), font=sub_font)
    img.save(path)


def create_terminal_frame(lines, path):
    img = Image.new("RGB", (WIDTH, HEIGHT), color=(22, 30, 40))
    draw = ImageDraw.Draw(img)
    margin = 40
    win_x0, win_y0 = margin, margin
    win_x1, win_y1 = WIDTH - margin, HEIGHT - margin
    draw.rounded_rectangle(
        (win_x0 + 6, win_y0 + 8, win_x1 + 6, win_y1 + 8),
        radius=12, fill=(0, 0, 0)
    )
    draw.rounded_rectangle(
        (win_x0, win_y0, win_x1, win_y1),
        radius=12, fill=(30, 30, 34)
    )
    bar_h = 36
    draw.rounded_rectangle(
        (win_x0, win_y0, win_x1, win_y0 + bar_h + 10),
        radius=12, fill=(55, 55, 60)
    )
    draw.rectangle(
        (win_x0, win_y0 + bar_h - 6, win_x1, win_y0 + bar_h + 10),
        fill=(30, 30, 34)
    )
    cx = win_x0 + 22
    cy = win_y0 + bar_h // 2 + 2
    for dx, colour in [(0, (255, 95, 87)), (22, (255, 189, 46)), (44, (39, 201, 63))]:
        draw.ellipse((cx + dx - 7, cy - 7, cx + dx + 7, cy + 7), fill=colour)
    title_font = get_font(13, bold=False)
    title = "pm"
    tb = draw.textbbox((0, 0), title, font=title_font)
    tw = tb[2] - tb[0]
    draw.text(((WIDTH - tw) // 2, win_y0 + 10), title, fill=(180, 180, 180), font=title_font)

    font = get_font(18, mono=True)
    y = win_y0 + bar_h + 28
    for text, color in lines:
        if color is None:
            y += 10
            continue
        draw.text((win_x0 + 24, y), text, fill=color, font=font)
        y += 24
    img.save(path)


def add_annotation(src, annotation, dst):
    img = Image.open(src).resize((WIDTH, HEIGHT), Image.LANCZOS).convert("RGBA")
    overlay = Image.new("RGBA", (WIDTH, HEIGHT), (0, 0, 0, 0))
    draw = ImageDraw.Draw(overlay)
    bar_h = 80
    draw.rectangle((0, HEIGHT - bar_h, WIDTH, HEIGHT), fill=(17, 17, 17, 230))
    font = get_font(22, bold=False)
    y = HEIGHT - bar_h + 12
    for line in annotation.split("\n"):
        lb = draw.textbbox((0, 0), line, font=font)
        lw = lb[2] - lb[0]
        draw.text(((WIDTH - lw) // 2, y), line, fill=(220, 220, 220, 255), font=font)
        y += 30
    composed = Image.alpha_composite(img, overlay)
    composed.convert("RGB").save(dst)


def run():
    FRAMES_DIR.mkdir(exist_ok=True)
    OUTPUT.parent.mkdir(exist_ok=True)
    for f in FRAMES_DIR.glob("*.png"):
        f.unlink()

    frame_num = 0
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page(viewport={"width": WIDTH, "height": HEIGHT})
        for scene in SCENES:
            label = scene.get("action", scene.get("title", "terminal"))
            print(f"  scene {frame_num}: {label}")
            if scene.get("is_title_card"):
                for _ in range(scene["duration"] * FPS):
                    create_title_card(scene["title"], scene["subtitle"], FRAMES_DIR / f"frame_{frame_num:04d}.png")
                    frame_num += 1
                continue
            if scene.get("is_terminal"):
                for _ in range(scene["duration"] * FPS):
                    fp = FRAMES_DIR / f"frame_{frame_num:04d}.png"
                    create_terminal_frame(scene["terminal_lines"], fp)
                    if scene.get("annotation"):
                        add_annotation(fp, scene["annotation"], fp)
                    frame_num += 1
                continue
            action = scene.get("action", "")
            html_file = ROOT / f"{action}.html"
            page.goto(f"file://{html_file}", wait_until="networkidle", timeout=10000)
            raw = FRAMES_DIR / f"raw_{frame_num:04d}.png"
            page.screenshot(path=str(raw), full_page=False)
            for _ in range(scene["duration"] * FPS):
                fp = FRAMES_DIR / f"frame_{frame_num:04d}.png"
                if scene.get("annotation"):
                    add_annotation(raw, scene["annotation"], fp)
                else:
                    Image.open(raw).resize((WIDTH, HEIGHT), Image.LANCZOS).save(fp)
                frame_num += 1
            raw.unlink(missing_ok=True)
        browser.close()

    print(f"generated {frame_num} frames, encoding video...")
    subprocess.run([
        "ffmpeg", "-y",
        "-framerate", str(FPS),
        "-i", str(FRAMES_DIR / "frame_%04d.png"),
        "-c:v", "libx264",
        "-pix_fmt", "yuv420p",
        "-r", "30",
        "-preset", "medium",
        "-crf", "23",
        str(OUTPUT),
    ], check=True)

    for f in FRAMES_DIR.glob("*.png"):
        f.unlink()
    FRAMES_DIR.rmdir()
    print(f"done. video at {OUTPUT}")


if __name__ == "__main__":
    run()

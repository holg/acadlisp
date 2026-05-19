#!/usr/bin/env python3
"""
make_video.py - Generate 10-minute MP4 from VIDEO.LSP output

"The AutoLISP Story: Automation in the Age of DOS"
A rust-autolisp Documentary

This script:
1. Takes the VIDEO.svg (all frames stacked vertically)
2. Splits it into individual frame images
3. Compiles them into a 10-minute MP4

Prerequisites:
    pip install cairosvg pillow

For video compilation:
    brew install ffmpeg  (macOS)
    apt install ffmpeg   (Linux)
"""

import os
import sys
import subprocess
import json
from pathlib import Path

# Try to import optional dependencies
try:
    from PIL import Image
    HAS_PIL = True
except ImportError:
    HAS_PIL = False

try:
    import cairosvg
    HAS_CAIROSVG = True
except ImportError:
    HAS_CAIROSVG = False


# Video settings
FRAME_WIDTH = 1920
FRAME_HEIGHT = 1080
FRAME_MARGIN = 20
FRAME_TOTAL_HEIGHT = FRAME_HEIGHT + FRAME_MARGIN

# Frame durations in seconds (matches VIDEO.LSP structure)
FRAME_DURATIONS = [
    # Scene 1: Title Sequence (30s)
    5, 5, 10, 10,
    # Scene 2: The Problem (60s)
    15, 15, 15, 15,
    # Scene 3: AutoLISP Introduction (75s)
    15, 20, 20, 20,
    # Scene 4: Type Safety + CSV (45s)
    15, 15, 20, 15,
    # Scene 5: Electrical Symbols (70s)
    15, 20, 15, 20,
    # Scene 6: Batch Processing (75s)
    15, 10, 10, 15, 20, 15,
    # Scene 7: The Legacy (80s)
    20, 20, 20, 20,
    # Scene 8: Conclusion (90s)
    20, 20, 30, 30,
]


def get_project_paths():
    """Get project directory paths"""
    script_dir = Path(__file__).parent
    project_dir = script_dir.parent
    output_dir = project_dir / "output"
    frames_dir = output_dir / "frames"

    return {
        "project": project_dir,
        "output": output_dir,
        "frames": frames_dir,
        "video_svg": output_dir / "VIDEO.svg",
        "video_json": output_dir / "VIDEO.json",
        "final_video": output_dir / "autolisp_story.mp4",
    }


def check_prerequisites():
    """Check if required tools are available"""
    print("Checking prerequisites...")

    issues = []

    # Check for ffmpeg
    try:
        subprocess.run(["ffmpeg", "-version"], capture_output=True, check=True)
        print("  ffmpeg: OK")
    except (subprocess.CalledProcessError, FileNotFoundError):
        issues.append("ffmpeg not found. Install with: brew install ffmpeg")

    # Check for image conversion capability
    if HAS_CAIROSVG:
        print("  cairosvg: OK")
    elif HAS_PIL:
        print("  PIL: OK (limited SVG support)")
    else:
        try:
            subprocess.run(["inkscape", "--version"], capture_output=True, check=True)
            print("  inkscape: OK")
        except (subprocess.CalledProcessError, FileNotFoundError):
            issues.append("No SVG converter found. Install: pip install cairosvg pillow")

    if issues:
        print("\nMissing prerequisites:")
        for issue in issues:
            print(f"  - {issue}")
        return False

    print()
    return True


def extract_frames_from_json(paths):
    """Extract frame data from VIDEO.json for creating individual frames"""
    json_path = paths["video_json"]

    if not json_path.exists():
        print(f"ERROR: {json_path} not found")
        return None

    with open(json_path) as f:
        data = json.load(f)

    # The JSON contains drawing entities
    # We need to group them by frame
    entities = data.get("entities", [])
    print(f"  Found {len(entities)} drawing entities")

    return entities


def convert_svg_to_frames_cairosvg(paths):
    """Convert VIDEO.svg to individual frame PNGs using cairosvg"""
    print("Converting VIDEO.svg to frames using cairosvg...")

    svg_path = paths["video_svg"]
    frames_dir = paths["frames"]
    frames_dir.mkdir(exist_ok=True)

    # Read SVG to get dimensions
    with open(svg_path, "rb") as f:
        svg_data = f.read()

    # Get SVG dimensions (parse viewBox or width/height)
    import re

    viewbox_match = re.search(rb'viewBox="([^"]+)"', svg_data)
    if viewbox_match:
        parts = viewbox_match.group(1).decode().split()
        if len(parts) == 4:
            total_width = float(parts[2])
            total_height = float(parts[3])
    else:
        # Try to get from width/height attributes
        total_height = 40000  # Default estimate

    num_frames = int(total_height // FRAME_TOTAL_HEIGHT)
    print(f"  Estimated {num_frames} frames")

    # Convert full SVG to PNG at high resolution
    print("  Converting full SVG to PNG...")
    full_png = frames_dir / "full_video.png"

    cairosvg.svg2png(
        url=str(svg_path),
        write_to=str(full_png),
        output_width=FRAME_WIDTH,
        output_height=int(total_height * FRAME_WIDTH / 1920),
    )

    # Split into individual frames using PIL
    if HAS_PIL:
        print("  Splitting into individual frames...")
        full_img = Image.open(full_png)
        actual_height = full_img.height
        num_frames = actual_height // FRAME_TOTAL_HEIGHT

        for i in range(num_frames):
            y_start = i * FRAME_TOTAL_HEIGHT
            y_end = y_start + FRAME_HEIGHT

            frame = full_img.crop((0, y_start, FRAME_WIDTH, y_end))
            frame_path = frames_dir / f"frame_{i+1:04d}.png"
            frame.save(frame_path)
            print(f"    Frame {i+1}/{num_frames}")

        # Clean up
        full_png.unlink()
        print(f"  Created {num_frames} frame images")
        return num_frames

    return 0


def convert_svg_to_frames_inkscape(paths):
    """Convert VIDEO.svg to individual frame PNGs using Inkscape"""
    print("Converting VIDEO.svg to frames using Inkscape...")

    svg_path = paths["video_svg"]
    frames_dir = paths["frames"]
    frames_dir.mkdir(exist_ok=True)

    # Get SVG dimensions using Inkscape
    result = subprocess.run(
        ["inkscape", "--query-height", str(svg_path)],
        capture_output=True,
        text=True,
    )
    total_height = int(float(result.stdout.strip()))

    num_frames = total_height // FRAME_TOTAL_HEIGHT
    print(f"  SVG height: {total_height}px")
    print(f"  Number of frames: {num_frames}")

    for i in range(num_frames):
        y_offset = i * FRAME_TOTAL_HEIGHT
        frame_path = frames_dir / f"frame_{i+1:04d}.png"

        print(f"  Exporting frame {i+1}/{num_frames}...")

        # Export specific area
        subprocess.run(
            [
                "inkscape",
                str(svg_path),
                "--export-type=png",
                f"--export-filename={frame_path}",
                f"--export-area=0:{y_offset}:{FRAME_WIDTH}:{y_offset + FRAME_HEIGHT}",
                f"--export-width={FRAME_WIDTH}",
                f"--export-height={FRAME_HEIGHT}",
            ],
            capture_output=True,
        )

    print(f"  Created {num_frames} frame images")
    return num_frames


def create_ffmpeg_concat_file(paths, num_frames):
    """Create ffmpeg concat demuxer file with frame durations"""
    concat_file = paths["frames"] / "frames.txt"

    with open(concat_file, "w") as f:
        for i, duration in enumerate(FRAME_DURATIONS[:num_frames]):
            frame_path = paths["frames"] / f"frame_{i+1:04d}.png"
            if frame_path.exists():
                f.write(f"file '{frame_path}'\n")
                f.write(f"duration {duration}\n")

        # FFmpeg requires the last frame to be listed again
        last_frame = paths["frames"] / f"frame_{num_frames:04d}.png"
        if last_frame.exists():
            f.write(f"file '{last_frame}'\n")

    total_duration = sum(FRAME_DURATIONS[:num_frames])
    print(f"  Total video duration: {total_duration}s ({total_duration//60}m {total_duration%60}s)")

    return concat_file, total_duration


def create_video(paths, num_frames):
    """Create MP4 video from frame images"""
    print("\nCreating video...")

    concat_file, total_duration = create_ffmpeg_concat_file(paths, num_frames)

    output_video = paths["final_video"]

    cmd = [
        "ffmpeg",
        "-y",  # Overwrite output
        "-f", "concat",
        "-safe", "0",
        "-i", str(concat_file),
        "-c:v", "libx264",
        "-preset", "slow",
        "-crf", "18",
        "-pix_fmt", "yuv420p",
        "-vf", f"scale={FRAME_WIDTH}:{FRAME_HEIGHT}:force_original_aspect_ratio=decrease,"
               f"pad={FRAME_WIDTH}:{FRAME_HEIGHT}:(ow-iw)/2:(oh-ih)/2:black",
        str(output_video),
    ]

    print("  Running ffmpeg...")
    result = subprocess.run(cmd, capture_output=True, text=True)

    if result.returncode != 0:
        print(f"  ERROR: ffmpeg failed")
        print(result.stderr)
        return False

    print(f"\n{'='*50}")
    print(f"  Video created: {output_video}")
    print(f"  Duration: {total_duration}s ({total_duration//60}m {total_duration%60}s)")
    print(f"{'='*50}")

    return True


def create_simple_html_slideshow(paths):
    """Create an HTML slideshow as fallback if video tools are not available"""
    print("\nCreating HTML slideshow fallback...")

    frames_dir = paths["frames"]
    output_html = paths["output"] / "video_slideshow.html"

    frames = sorted(frames_dir.glob("frame_*.png"))
    if not frames:
        print("  No frames found, using VIDEO.svg directly")
        # Use the combined SVG with CSS animation
        html_content = f'''<!DOCTYPE html>
<html>
<head>
    <title>The AutoLISP Story</title>
    <style>
        body {{ margin: 0; background: #000; overflow: hidden; }}
        .container {{
            width: 100vw;
            height: 100vh;
            overflow: hidden;
        }}
        img {{
            width: 1920px;
            animation: scroll-video {sum(FRAME_DURATIONS)}s linear infinite;
        }}
        @keyframes scroll-video {{
            0% {{ transform: translateY(0); }}
            100% {{ transform: translateY(-{len(FRAME_DURATIONS) * FRAME_TOTAL_HEIGHT}px); }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <img src="VIDEO.svg" alt="The AutoLISP Story">
    </div>
</body>
</html>'''
    else:
        # Create slideshow with individual frames
        slides_html = ""
        for i, frame in enumerate(frames):
            duration = FRAME_DURATIONS[i] if i < len(FRAME_DURATIONS) else 10
            slides_html += f'        <img src="frames/{frame.name}" data-duration="{duration}">\n'

        html_content = f'''<!DOCTYPE html>
<html>
<head>
    <title>The AutoLISP Story - Documentary</title>
    <style>
        body {{
            margin: 0;
            background: #000;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
        }}
        .slideshow {{
            position: relative;
            width: 1920px;
            height: 1080px;
            max-width: 100vw;
            max-height: 100vh;
        }}
        .slideshow img {{
            position: absolute;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            object-fit: contain;
            opacity: 0;
            transition: opacity 0.5s;
        }}
        .slideshow img.active {{
            opacity: 1;
        }}
        .controls {{
            position: fixed;
            bottom: 20px;
            left: 50%;
            transform: translateX(-50%);
            background: rgba(0,0,0,0.8);
            padding: 10px 20px;
            border-radius: 5px;
            color: #fff;
            font-family: monospace;
        }}
        .progress {{
            width: 400px;
            height: 4px;
            background: #333;
            margin-top: 10px;
        }}
        .progress-bar {{
            height: 100%;
            background: #0f0;
            width: 0%;
            transition: width 0.1s linear;
        }}
    </style>
</head>
<body>
    <div class="slideshow" id="slideshow">
{slides_html}    </div>
    <div class="controls">
        <span id="status">Frame 1/{len(frames)}</span>
        <div class="progress">
            <div class="progress-bar" id="progress"></div>
        </div>
    </div>
    <script>
        const images = document.querySelectorAll('#slideshow img');
        const status = document.getElementById('status');
        const progress = document.getElementById('progress');
        let current = 0;

        function showSlide(index) {{
            images.forEach((img, i) => {{
                img.classList.toggle('active', i === index);
            }});
            status.textContent = `Frame ${{index + 1}}/${{images.length}}`;

            const duration = parseInt(images[index].dataset.duration) * 1000;
            progress.style.transition = 'none';
            progress.style.width = '0%';

            setTimeout(() => {{
                progress.style.transition = `width ${{duration}}ms linear`;
                progress.style.width = '100%';
            }}, 50);

            setTimeout(() => {{
                current = (current + 1) % images.length;
                showSlide(current);
            }}, duration);
        }}

        // Start slideshow
        showSlide(0);
    </script>
</body>
</html>'''

    with open(output_html, "w") as f:
        f.write(html_content)

    print(f"  Created: {output_html}")


def main():
    print("=" * 50)
    print("  THE AUTOLISP STORY - Video Generator")
    print("  10-minute documentary")
    print("=" * 50)
    print()

    paths = get_project_paths()

    # Check prerequisites
    if not check_prerequisites():
        print("Some prerequisites are missing.")
        print("Creating HTML slideshow instead...")
        create_simple_html_slideshow(paths)
        return

    # Check if VIDEO.svg exists
    if not paths["video_svg"].exists():
        print(f"ERROR: VIDEO.svg not found at {paths['video_svg']}")
        print("Run: cargo run -- samples/VIDEO.LSP -o output/VIDEO")
        return

    # Convert SVG to frames
    paths["frames"].mkdir(exist_ok=True)

    if HAS_CAIROSVG and HAS_PIL:
        num_frames = convert_svg_to_frames_cairosvg(paths)
    else:
        try:
            num_frames = convert_svg_to_frames_inkscape(paths)
        except FileNotFoundError:
            print("ERROR: No SVG converter available")
            create_simple_html_slideshow(paths)
            return

    if num_frames == 0:
        print("ERROR: No frames generated")
        return

    # Create video
    if create_video(paths, num_frames):
        print(f"\nTo play: open {paths['final_video']}")
    else:
        print("\nVideo creation failed. Creating HTML slideshow instead...")
        create_simple_html_slideshow(paths)


if __name__ == "__main__":
    main()

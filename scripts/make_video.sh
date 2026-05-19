#!/bin/bash
#
# make_video.sh - Generate 10-minute MP4 from VIDEO.LSP frames
#
# "The AutoLISP Story: Automation in the Age of DOS"
# A rust-autolisp Documentary
#
# Prerequisites:
#   - Inkscape (for SVG to PNG conversion)
#   - ffmpeg (for video encoding)
#
# Install on macOS:
#   brew install inkscape ffmpeg
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="$PROJECT_DIR/output"
FRAMES_DIR="$OUTPUT_DIR/frames"
VIDEO_SVG="$OUTPUT_DIR/VIDEO.svg"
FINAL_VIDEO="$OUTPUT_DIR/autolisp_story.mp4"

# Video settings
FRAME_WIDTH=1920
FRAME_HEIGHT=1080
FPS=1  # 1 frame per second for slideshow-style video

echo "============================================"
echo "  THE AUTOLISP STORY - Video Generator"
echo "  10-minute documentary"
echo "============================================"
echo ""

# Check prerequisites
check_command() {
    if ! command -v "$1" &> /dev/null; then
        echo "ERROR: $1 is not installed."
        echo "Install with: brew install $1"
        exit 1
    fi
}

echo "Checking prerequisites..."
check_command inkscape
check_command ffmpeg
echo "  All prerequisites found!"
echo ""

# Create frames directory
mkdir -p "$FRAMES_DIR"

# Frame timing data (frame_number:duration_seconds)
# This defines how long each frame should be shown
declare -a FRAME_DURATIONS=(
    # Scene 1: Title (30s total)
    "1:5"   # Title
    "2:5"   # Subtitle
    "3:10"  # Era establishing
    "4:10"  # The office

    # Scene 2: The Problem (60s total)
    "5:15"  # Stack of drawings
    "6:15"  # Time problem
    "7:15"  # The cost
    "8:15"  # The idea

    # Scene 3: AutoLISP Introduction (90s total)
    "9:15"   # What is AutoLISP
    "10:20"  # LISP syntax
    "11:20"  # Building blocks
    "12:20"  # The (load) command
    "13:15"  # Strong typing

    # Scene 4: The CSV Data (60s total)
    "14:15"  # Where does data come from
    "15:20"  # CSV structure
    "16:15"  # German CSV format
    "17:10"  # DIN Standard intro

    # Scene 5: Electrical Symbols (90s total)
    "17:15"  # DIN Standard
    "18:20"  # Schuetz detail
    "19:15"  # Motor symbol
    "20:20"  # All symbols overview

    # Scene 6: Batch Processing (90s total)
    "21:15"  # Magic command
    "22:10"  # Processing frame 1
    "23:10"  # Processing frame 2
    "24:15"  # Complete!
    "25:20"  # Output files
    "26:15"  # Plotting

    # Scene 7: Legacy (90s total)
    "27:20"  # Impact
    "28:20"  # Spread
    "29:20"  # Evolution
    "30:20"  # Recreation

    # Scene 8: Conclusion (90s total)
    "31:20"  # Lessons learned
    "32:20"  # Thank you
    "33:30"  # Credits
    "34:30"  # Final logo
)

# Method 1: If we have individual frame SVGs, convert each
convert_individual_frames() {
    echo "Converting individual frame SVGs to PNG..."

    for svg_file in "$FRAMES_DIR"/frame_*.svg; do
        if [ -f "$svg_file" ]; then
            png_file="${svg_file%.svg}.png"
            echo "  Converting: $(basename "$svg_file")"
            inkscape "$svg_file" --export-type=png --export-filename="$png_file" \
                --export-width=$FRAME_WIDTH --export-height=$FRAME_HEIGHT
        fi
    done
}

# Method 2: Split the combined VIDEO.svg into individual frames
# The VIDEO.svg contains all frames stacked vertically
split_combined_svg() {
    echo "Splitting combined VIDEO.svg into individual frame PNGs..."

    if [ ! -f "$VIDEO_SVG" ]; then
        echo "ERROR: VIDEO.svg not found at $VIDEO_SVG"
        echo "Run: cargo run -- samples/VIDEO.LSP -o output/VIDEO"
        exit 1
    fi

    # Get total height of SVG
    TOTAL_HEIGHT=$(inkscape --query-height "$VIDEO_SVG" 2>/dev/null | cut -d'.' -f1)

    # Calculate number of frames (each frame is 1100 pixels high: 1080 + 20 margin)
    FRAME_TOTAL_HEIGHT=$((FRAME_HEIGHT + 20))
    NUM_FRAMES=$((TOTAL_HEIGHT / FRAME_TOTAL_HEIGHT))

    echo "  Total SVG height: ${TOTAL_HEIGHT}px"
    echo "  Frame height: ${FRAME_TOTAL_HEIGHT}px"
    echo "  Number of frames: $NUM_FRAMES"
    echo ""

    # Export each frame as a separate PNG
    for ((i=1; i<=NUM_FRAMES; i++)); do
        Y_OFFSET=$(( (i-1) * FRAME_TOTAL_HEIGHT ))
        FRAME_FILE="$FRAMES_DIR/frame_$(printf '%04d' $i).png"

        echo "  Exporting frame $i/$NUM_FRAMES (y=$Y_OFFSET)..."

        # Use Inkscape to export a specific area
        inkscape "$VIDEO_SVG" \
            --export-type=png \
            --export-filename="$FRAME_FILE" \
            --export-area="0:$Y_OFFSET:$FRAME_WIDTH:$((Y_OFFSET + FRAME_HEIGHT))" \
            --export-width=$FRAME_WIDTH \
            --export-height=$FRAME_HEIGHT \
            2>/dev/null
    done

    echo ""
    echo "  Frames exported: $NUM_FRAMES"
}

# Create video with proper frame durations
create_video_with_durations() {
    echo ""
    echo "Creating video with frame durations..."

    # Create a temporary file listing all frames with their durations
    CONCAT_FILE="$FRAMES_DIR/frames.txt"
    > "$CONCAT_FILE"

    FRAME_INDEX=1
    TOTAL_SECONDS=0

    for timing in "${FRAME_DURATIONS[@]}"; do
        FRAME_NUM=$(echo "$timing" | cut -d':' -f1)
        DURATION=$(echo "$timing" | cut -d':' -f2)

        FRAME_FILE="$FRAMES_DIR/frame_$(printf '%04d' $FRAME_NUM).png"

        if [ -f "$FRAME_FILE" ]; then
            echo "file '$FRAME_FILE'" >> "$CONCAT_FILE"
            echo "duration $DURATION" >> "$CONCAT_FILE"
            TOTAL_SECONDS=$((TOTAL_SECONDS + DURATION))
        fi

        FRAME_INDEX=$((FRAME_INDEX + 1))
    done

    # Add the last frame again (ffmpeg requirement)
    LAST_FRAME="$FRAMES_DIR/frame_$(printf '%04d' ${#FRAME_DURATIONS[@]}).png"
    if [ -f "$LAST_FRAME" ]; then
        echo "file '$LAST_FRAME'" >> "$CONCAT_FILE"
    fi

    echo "  Total video duration: ${TOTAL_SECONDS}s ($((TOTAL_SECONDS / 60)) min $((TOTAL_SECONDS % 60)) sec)"
    echo ""

    # Create video using concat demuxer
    echo "Encoding video..."
    ffmpeg -y -f concat -safe 0 -i "$CONCAT_FILE" \
        -c:v libx264 \
        -preset slow \
        -crf 18 \
        -pix_fmt yuv420p \
        -vf "scale=${FRAME_WIDTH}:${FRAME_HEIGHT}:force_original_aspect_ratio=decrease,pad=${FRAME_WIDTH}:${FRAME_HEIGHT}:(ow-iw)/2:(oh-ih)/2:black" \
        "$FINAL_VIDEO"

    echo ""
    echo "============================================"
    echo "  Video created: $FINAL_VIDEO"
    echo "  Duration: ${TOTAL_SECONDS}s"
    echo "============================================"
}

# Simple video creation (all frames same duration)
create_simple_video() {
    echo ""
    echo "Creating video from PNG frames..."

    # Count frames
    FRAME_COUNT=$(ls -1 "$FRAMES_DIR"/frame_*.png 2>/dev/null | wc -l | tr -d ' ')

    if [ "$FRAME_COUNT" -eq 0 ]; then
        echo "ERROR: No frame files found in $FRAMES_DIR"
        exit 1
    fi

    echo "  Found $FRAME_COUNT frames"

    # Calculate duration per frame to get 10 minutes (600 seconds)
    TARGET_DURATION=600
    DURATION_PER_FRAME=$((TARGET_DURATION / FRAME_COUNT))

    if [ "$DURATION_PER_FRAME" -lt 1 ]; then
        DURATION_PER_FRAME=1
    fi

    echo "  Duration per frame: ${DURATION_PER_FRAME}s"
    echo "  Total duration: $((FRAME_COUNT * DURATION_PER_FRAME))s"
    echo ""

    # Create video
    echo "Encoding video..."
    ffmpeg -y -framerate "1/$DURATION_PER_FRAME" \
        -i "$FRAMES_DIR/frame_%04d.png" \
        -c:v libx264 \
        -preset slow \
        -crf 18 \
        -pix_fmt yuv420p \
        -vf "scale=${FRAME_WIDTH}:${FRAME_HEIGHT}:force_original_aspect_ratio=decrease,pad=${FRAME_WIDTH}:${FRAME_HEIGHT}:(ow-iw)/2:(oh-ih)/2:black" \
        "$FINAL_VIDEO"

    echo ""
    echo "============================================"
    echo "  Video created: $FINAL_VIDEO"
    echo "  Duration: $((FRAME_COUNT * DURATION_PER_FRAME))s"
    echo "============================================"
}

# Main execution
echo "Step 1: Generating frame images..."
split_combined_svg

echo ""
echo "Step 2: Compiling video..."

# Check if we have duration-controlled frames
if [ ${#FRAME_DURATIONS[@]} -gt 0 ] && [ -f "$FRAMES_DIR/frame_0001.png" ]; then
    create_video_with_durations
else
    create_simple_video
fi

echo ""
echo "Done! Open the video with:"
echo "  open $FINAL_VIDEO"
echo ""

#!/usr/bin/env python3
"""
Generate logo.png with LSB steganography key embedding.
The key is hidden in the LSB of red channel pixels.

Usage:
    python3 generate_logo.py [key]

Default key: HN2025
Output: dist/logo.png
"""

import sys
import struct
import zlib

# Simple PNG writer (no dependencies needed)
def create_png(width, height, pixels):
    """Create a PNG file from RGBA pixel data."""
    def png_chunk(chunk_type, data):
        chunk = chunk_type + data
        return struct.pack('>I', len(data)) + chunk + struct.pack('>I', zlib.crc32(chunk) & 0xffffffff)

    # PNG signature
    signature = b'\x89PNG\r\n\x1a\n'

    # IHDR chunk
    ihdr_data = struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0)  # 8-bit RGBA
    ihdr = png_chunk(b'IHDR', ihdr_data)

    # IDAT chunk (image data)
    raw_data = b''
    for y in range(height):
        raw_data += b'\x00'  # Filter byte (none)
        for x in range(width):
            idx = (y * width + x) * 4
            raw_data += bytes(pixels[idx:idx+4])

    compressed = zlib.compress(raw_data, 9)
    idat = png_chunk(b'IDAT', compressed)

    # IEND chunk
    iend = png_chunk(b'IEND', b'')

    return signature + ihdr + idat + iend

def embed_key_lsb(pixels, width, height, key):
    """Embed key in LSB of red channel pixels."""
    # Convert key to bits
    key_bytes = key.encode('utf-8')
    # Add length prefix (2 bytes) and null terminator
    data = struct.pack('>H', len(key_bytes)) + key_bytes + b'\x00'
    bits = []
    for byte in data:
        for i in range(8):
            bits.append((byte >> (7 - i)) & 1)

    # Embed bits in LSB of red channel
    bit_idx = 0
    for i in range(0, len(pixels), 4):  # RGBA, step by 4
        if bit_idx >= len(bits):
            break
        # Modify LSB of red channel
        pixels[i] = (pixels[i] & 0xFE) | bits[bit_idx]
        bit_idx += 1

    print(f"Embedded {len(bits)} bits ({len(data)} bytes) into {bit_idx} pixels")
    return pixels

def generate_acadlisp_logo(width=128, height=128):
    """Generate a simple AcadLISP logo."""
    pixels = []

    # Colors
    bg = (26, 26, 46, 255)        # Dark blue background #1a1a2e
    accent = (0, 255, 136, 255)   # Green accent #00ff88
    orange = (255, 136, 0, 255)  # Orange #ff8800

    cx, cy = width // 2, height // 2

    for y in range(height):
        for x in range(width):
            # Distance from center
            dx, dy = x - cx, y - cy
            dist = (dx*dx + dy*dy) ** 0.5

            # Create a stylized "A" shape for AcadLISP
            pixel = bg

            # Outer ring
            if 35 < dist < 45:
                pixel = accent

            # Inner "A" shape
            if 20 < dist < 30:
                angle = __import__('math').atan2(dy, dx)
                if -2.5 < angle < -0.6 or 0.6 < angle < 2.5:  # Sides of A
                    pixel = orange

            # Center dot
            if dist < 8:
                pixel = accent

            # Crossbar of A
            if abs(dy + 5) < 4 and abs(dx) < 15:
                pixel = orange

            pixels.extend(pixel)

    return pixels

def main():
    key = sys.argv[1] if len(sys.argv) > 1 else "HN2025"
    width, height = 128, 128

    print(f"Generating logo with key: {key}")

    # Generate logo pixels
    pixels = generate_acadlisp_logo(width, height)

    # Embed key using LSB
    pixels = embed_key_lsb(pixels, width, height, key)

    # Create PNG
    png_data = create_png(width, height, pixels)

    # Write to dist/logo.png
    import os
    os.makedirs('dist', exist_ok=True)
    output_path = 'dist/logo.png'
    with open(output_path, 'wb') as f:
        f.write(png_data)

    print(f"Logo saved to {output_path} ({len(png_data)} bytes)")
    print(f"Key '{key}' embedded via LSB steganography")

if __name__ == '__main__':
    main()

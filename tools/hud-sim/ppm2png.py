"""Convert a P6 PPM to PNG (2x nearest-neighbour upscale). Stdlib only."""
import struct, sys, zlib

def main(src, dst, scale=2):
    data = open(src, "rb").read()
    parts = data.split(b"\n", 3)
    w, h = map(int, parts[1].split())
    px = parts[3]
    rows = []
    for y in range(h):
        row = px[y * w * 3:(y + 1) * w * 3]
        big = b"".join(row[x * 3:x * 3 + 3] * scale for x in range(w))
        rows += [b"\x00" + big] * scale
    def chunk(t, d):
        return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w * scale, h * scale, 8, 2, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(b"".join(rows), 9)) + chunk(b"IEND", b"")
    open(dst, "wb").write(png)

if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])

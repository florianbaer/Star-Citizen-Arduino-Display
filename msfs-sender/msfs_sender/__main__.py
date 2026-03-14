"""MSFS 2024 attitude sender - reads SimConnect data and sends to ESP32 display."""

import argparse
import time

import serial

from .protocol import frame_attitude
from .simconnect_source import AttitudeSource


def main():
    parser = argparse.ArgumentParser(
        description="Send MSFS 2024 attitude data to ESP32 gyroscope display"
    )
    parser.add_argument("port", help="Serial port (e.g. COM6, /dev/ttyUSB0)")
    parser.add_argument("--baud", type=int, default=115200, help="Baud rate (default: 115200)")
    parser.add_argument("--hz", type=int, default=20, help="Send rate in Hz (default: 20)")
    args = parser.parse_args()

    ser = serial.Serial(args.port, args.baud)
    source = AttitudeSource()
    interval = 1.0 / args.hz

    print(f"Connected to {args.port} at {args.baud} baud, sending at {args.hz}Hz")
    print("Press Ctrl+C to stop")

    try:
        while True:
            pitch, roll, heading = source.read()
            frame = frame_attitude(pitch, roll, heading)
            ser.write(frame)
            time.sleep(interval)
    except KeyboardInterrupt:
        print("\nStopping...")
        source.close()
        ser.close()


if __name__ == "__main__":
    main()

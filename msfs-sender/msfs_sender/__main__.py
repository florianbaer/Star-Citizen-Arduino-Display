"""MSFS 2024 attitude sender - reads SimConnect data and sends to ESP32 display."""

import argparse
import sys
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

    try:
        ser = serial.Serial(args.port, args.baud)
    except serial.SerialException as e:
        print(f"Failed to open serial port {args.port}: {e}", file=sys.stderr)
        sys.exit(1)

    try:
        source = AttitudeSource()
    except Exception as e:
        ser.close()
        print(f"Failed to connect to MSFS SimConnect: {e}", file=sys.stderr)
        print("Make sure MSFS 2024 is running and you are in a flight.", file=sys.stderr)
        sys.exit(1)

    interval = 1.0 / args.hz
    print(f"Connected to {args.port} at {args.baud} baud, sending at {args.hz}Hz")
    print("Press Ctrl+C to stop")

    try:
        while True:
            try:
                pitch, roll, heading = source.read()
            except Exception as e:
                print(f"SimConnect read error: {e}", file=sys.stderr)
                time.sleep(1)
                continue

            frame = frame_attitude(pitch, roll, heading)
            try:
                ser.write(frame)
            except serial.SerialException as e:
                print(f"Serial write error: {e}", file=sys.stderr)
                break
            time.sleep(interval)
    except KeyboardInterrupt:
        print("\nStopping...")
    finally:
        try:
            source.close()
        except Exception:
            pass
        ser.close()


if __name__ == "__main__":
    main()

"""MSFS 2024 data sender - reads SimConnect data and sends to ESP32 display."""

import argparse
import sys
import time

import serial

from .protocol import frame_attitude, frame_engine, frame_flight_data, frame_gforce
from .simconnect_source import MsfsSource


def main():
    parser = argparse.ArgumentParser(
        description="Send MSFS 2024 flight data to ESP32 display"
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
        source = MsfsSource()
    except Exception as e:
        ser.close()
        print(f"Failed to connect to MSFS SimConnect: {e}", file=sys.stderr)
        print("Make sure MSFS 2024 is running and you are in a flight.", file=sys.stderr)
        sys.exit(1)

    interval = 1.0 / args.hz
    print(f"Connected to {args.port} at {args.baud} baud, sending at {args.hz}Hz")
    print("Sending: attitude, engine, flight data, G-force")
    print("Press Ctrl+C to stop")

    try:
        while True:
            try:
                pitch, roll, heading = source.read_attitude()
                ser.write(frame_attitude(pitch, roll, heading))

                rpm, throttle, ff, ot, op = source.read_engine()
                ser.write(frame_engine(rpm, throttle, ff, ot, op))

                ias, alt, vs, gs = source.read_flight_data()
                ser.write(frame_flight_data(ias, alt, vs, gs))

                gx, gy, gz = source.read_gforce()
                ser.write(frame_gforce(gx, gy, gz))
            except serial.SerialException as e:
                print(f"Serial write error: {e}", file=sys.stderr)
                break
            except Exception as e:
                print(f"SimConnect read error: {e}", file=sys.stderr)
                time.sleep(1)
                continue

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

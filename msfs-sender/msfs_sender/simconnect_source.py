"""SimConnect wrapper for reading MSFS 2024 attitude data."""

import math


class AttitudeSource:
    """Reads pitch, roll, and heading from MSFS via SimConnect."""

    def __init__(self):
        # Lazy import: SimConnect is Windows-only
        from SimConnect import SimConnect, AircraftRequests

        self._sm = SimConnect()
        self._aq = AircraftRequests(self._sm, _time=0)

    def read(self) -> tuple[int, int, int]:
        """Read current attitude from MSFS.

        Returns:
            (pitch_tenths, roll_tenths, heading_tenths) as integers.
            Pitch/roll in range [-1800, 1800], heading in [0, 3599].
        """
        # SimConnect returns radians despite variable names containing "DEGREES"
        pitch_rad = self._aq.get("PLANE_PITCH_DEGREES") or 0.0
        roll_rad = self._aq.get("PLANE_BANK_DEGREES") or 0.0
        heading_rad = self._aq.get("PLANE_HEADING_DEGREES_TRUE") or 0.0

        pitch_deg = math.degrees(pitch_rad)
        roll_deg = math.degrees(roll_rad)
        heading_deg = math.degrees(heading_rad)

        pitch_t = max(-1800, min(1800, int(pitch_deg * 10)))
        roll_t = max(-1800, min(1800, int(roll_deg * 10)))
        heading_t = int(heading_deg * 10) % 3600

        return pitch_t, roll_t, heading_t

    def close(self):
        self._sm.exit()

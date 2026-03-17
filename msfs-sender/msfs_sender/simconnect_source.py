"""SimConnect wrapper for reading MSFS 2024 flight data."""

import math


class MsfsSource:
    """Reads flight data from MSFS via SimConnect."""

    def __init__(self):
        # Lazy import: SimConnect is Windows-only
        from SimConnect import SimConnect, AircraftRequests

        self._sm = SimConnect()
        self._aq = AircraftRequests(self._sm, _time=0)

    def read_attitude(self) -> tuple[int, int, int]:
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

    def read_engine(self) -> tuple[int, int, int, int, int]:
        """Read engine data from MSFS.

        Returns:
            (rpm, throttle, fuel_flow, oil_temp, oil_press)
            rpm: 0-65535, throttle: 0-100, others: 0-255
        """
        rpm_raw = self._aq.get("GENERAL_ENG_RPM:1") or 0.0
        throttle_raw = self._aq.get("GENERAL_ENG_THROTTLE_LEVER_POSITION:1") or 0.0
        ff_raw = self._aq.get("ENG_FUEL_FLOW_GPH:1") or 0.0
        ot_raw = self._aq.get("GENERAL_ENG_OIL_TEMPERATURE:1") or 0.0
        op_raw = self._aq.get("GENERAL_ENG_OIL_PRESSURE:1") or 0.0

        rpm = max(0, min(65535, int(rpm_raw)))
        throttle = max(0, min(100, int(throttle_raw)))
        # Map fuel flow (0-50 GPH typical) to 0-255
        fuel_flow = max(0, min(255, int(ff_raw * 255 / 50)))
        # Map oil temp (0-250°F typical) to 0-255
        oil_temp = max(0, min(255, int(ot_raw * 255 / 250)))
        # Map oil pressure (0-100 PSI typical) to 0-255
        oil_press = max(0, min(255, int(op_raw * 255 / 100)))

        return rpm, throttle, fuel_flow, oil_temp, oil_press

    def read_flight_data(self) -> tuple[int, int, int, int]:
        """Read flight data from MSFS.

        Returns:
            (airspeed, altitude, vspeed, ground_speed)
            airspeed/ground_speed in tenths of knots, altitude in feet, vspeed in fpm.
        """
        ias_raw = self._aq.get("AIRSPEED_INDICATED") or 0.0
        alt_raw = self._aq.get("INDICATED_ALTITUDE") or 0.0
        vs_raw = self._aq.get("VERTICAL_SPEED") or 0.0
        gs_raw = self._aq.get("GROUND_VELOCITY") or 0.0

        airspeed = max(0, min(65535, int(ias_raw * 10)))
        altitude = max(-2147483648, min(2147483647, int(alt_raw)))
        vspeed = max(-32768, min(32767, int(vs_raw)))
        ground_speed = max(0, min(65535, int(gs_raw * 10)))

        return airspeed, altitude, vspeed, ground_speed

    def read_gforce(self) -> tuple[int, int, int]:
        """Read G-force data from MSFS.

        Returns:
            (gx, gy, gz) in hundredths of G.
            gy ~100 in level flight (1.0G normal load).
        """
        # SimConnect acceleration vars return ft/s²
        ax_raw = self._aq.get("ACCELERATION_BODY_X") or 0.0
        ay_raw = self._aq.get("ACCELERATION_BODY_Y") or 0.0
        az_raw = self._aq.get("ACCELERATION_BODY_Z") or 0.0

        # Convert ft/s² to G (1G = 32.174 ft/s²), then to hundredths
        g_conv = 32.174
        gx = max(-32768, min(32767, int(ax_raw / g_conv * 100)))
        gy = max(-32768, min(32767, int(ay_raw / g_conv * 100)))
        gz = max(-32768, min(32767, int(az_raw / g_conv * 100)))

        return gx, gy, gz

    def close(self):
        self._sm.exit()


# Backwards-compatible alias
AttitudeSource = MsfsSource

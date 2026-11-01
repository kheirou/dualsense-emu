namespace DualSenseEmu.Core;

[Flags]
public enum DsButtons : ushort
{
    None = 0,
    Square = 1 << 0, Cross = 1 << 1, Circle = 1 << 2, Triangle = 1 << 3,
    L1 = 1 << 4, R1 = 1 << 5, Create = 1 << 6, Options = 1 << 7,
    L3 = 1 << 8, R3 = 1 << 9, Ps = 1 << 10, Touchpad = 1 << 11, Mute = 1 << 12,
    DpadUp = 1 << 13, DpadDown = 1 << 14, DpadLeft = 1 << 15,
    // DpadRight does not fit in 16 bits; see DualSenseState.DpadRight.
}

/// <summary>Raw DualSense state: sticks/triggers 0-255 (sticks centre at 128, up = 0).</summary>
public readonly record struct DualSenseState(
    byte Lx, byte Ly, byte Rx, byte Ry, byte L2, byte R2,
    DsButtons Buttons, bool DpadRight,
    int BatteryPercent = -1, ChargeState Charge = ChargeState.Unknown);

public enum ChargeState { Unknown, Discharging, Charging, Full }

"""Install the RC7 NAV and MOUSE bindings on an existing Vial keymap."""

import argparse
from pathlib import Path
import re
import sys
import tomllib


HERE = Path(__file__).resolve().parent
KEYCODES = {
    "Insert": 0x49, "Home": 0x4A, "PageUp": 0x4B, "Delete": 0x4C,
    "End": 0x4D, "PageDown": 0x4E, "Right": 0x4F, "Left": 0x50,
    "Down": 0x51, "Up": 0x52, "MouseUp": 0xCD, "MouseDown": 0xCE,
    "MouseLeft": 0xCF, "MouseRight": 0xD0, "MouseBtn1": 0xD1,
    "MouseBtn2": 0xD2, "MouseBtn3": 0xD3, "MouseWheelUp": 0xD9,
    "MouseWheelDown": 0xDA, "MouseWheelLeft": 0xDB, "MouseWheelRight": 0xDC,
}


def bindings(config):
    cfg = tomllib.loads(config.read_text(encoding="utf-8"))
    positions = [tuple(map(int, match)) for match in re.findall(r"\((\d+),(\d+)\)", cfg["layout"]["map"])]
    for layer, name in [(6, "NAV"), (7, "MOUSE")]:
        data = cfg["keymap"]["layer"][layer]
        if data["name"] != name:
            raise ValueError(f"Expected the RC7 {name} layer in keyboard.toml")
        for (row, col), key in zip(positions, data["keys"].split(), strict=True):
            if key != "_":
                yield (layer, row, col), 0x0001, KEYCODES[key]
    yield (0, 4, 4), 0x002C, 0x462C  # Space / LT(6, KC_SPC)
    yield (0, 9, 4), 0x002A, 0x472A  # Backspace / LT(7, KC_BSPC)


def read_key(query, position):
    header = bytes((0x04, *position))
    reply = query(header)
    if reply[:4] != header:
        raise OSError("Unexpected Vial keymap reply. Close Vial and WebHID.")
    return int.from_bytes(reply[4:6], "big")


def install(query, preset, apply):
    changes = []
    conflicts = []
    for position, original, desired in preset:
        existing = read_key(query, position)
        if existing == original:
            changes.append((position, desired))
        elif existing != desired:
            conflicts.append(position)
    if conflicts:
        coords = ", ".join(f"L{layer} ({row},{col})" for layer, row, col in conflicts)
        raise ValueError(f"Preset conflicts with existing assignments at {coords}. No keys changed.")
    if apply:
        for position, keycode in changes:
            packet = bytes((0x05, *position)) + keycode.to_bytes(2, "big")
            if query(packet)[:6] != packet or read_key(query, position) != keycode:
                raise OSError(f"Could not verify {position}. Re-run the installer to resume.")
    return len(changes)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--apply", action="store_true", help="write only compatible assignments")
    parser.add_argument("--config", type=Path, default=HERE.parents[1] / "firmware" / "keyboard.toml")
    args = parser.parse_args()
    maintenance = HERE.parent / "NRF-DFU"
    sys.path[:0] = [str(maintenance), str(maintenance / "lib")]
    import hid
    from enter_dfu import UID, devices, query

    try:
        found = [device for device in devices(hid) if device["product_id"] in (0x5350, 0x5351)]
        if len(found) != 1:
            raise ValueError("Connect exactly one PandaKB Vial interface (LEFT or RECEIVER).")
        handle = hid.device()
        handle.open_path(found[0]["path"])
        try:
            def request(packet):
                return query(handle, packet)

            if request([0xFE, 0])[4:12] != UID:
                raise ValueError("Unexpected Vial keyboard identity.")
            count = install(request, bindings(args.config), args.apply)
        finally:
            handle.close()
    except (ValueError, OSError) as error:
        print(error, file=sys.stderr)
        return 1
    print(f"Verified {count} new bindings." if args.apply else f"Ready to install {count} bindings.")
    if args.apply:
        print("Hold left thumb Space for NAV or right thumb Backspace for MOUSE. Reopen Vial to see the changes.")
    return 0


if __name__ == "__main__":
    sys.exit(main())

"""Build the three RC8 roles and package nRF52840 Adafruit UF2 images."""

import argparse
import hashlib
import os
from pathlib import Path
import struct
import subprocess


ROOT = Path(__file__).resolve().parents[1]
APP_START = 0x1000
APP_END = 0xE0000
UF2_FAMILY = 0xADA52840


def load_hex(path):
    memory = {}
    base = 0
    eof = False
    for line in path.read_text(encoding="ascii").splitlines():
        if eof or not line.startswith(":"):
            raise ValueError("Malformed Intel HEX")
        record = bytes.fromhex(line[1:])
        if len(record) < 5 or record[0] != len(record) - 5 or sum(record) % 256:
            raise ValueError("Invalid Intel HEX length or checksum")
        address = int.from_bytes(record[1:3], "big")
        kind = record[3]
        data = record[4:-1]
        if kind == 0:
            for offset, byte in enumerate(data):
                target = base + address + offset
                if target in memory or not APP_START <= target < APP_END:
                    raise ValueError(f"Unsafe or duplicate application address {target:#x}")
                memory[target] = byte
        elif kind in (2, 4):
            if len(data) != 2:
                raise ValueError("Malformed extended address")
            base = int.from_bytes(data, "big") << (4 if kind == 2 else 16)
        elif kind == 1:
            eof = True
        elif kind not in (3, 5):
            raise ValueError(f"Unsupported Intel HEX record {kind}")
    if not eof or not memory or min(memory) != APP_START:
        raise ValueError("Missing Intel HEX application data")
    vector = bytes(memory[address] for address in range(APP_START, APP_START + 8))
    stack, reset = struct.unpack("<II", vector)
    if not 0x20000000 < stack <= 0x20040000 or not reset & 1 or not APP_START <= (reset & ~1) < max(memory):
        raise ValueError("Invalid Cortex-M application vectors")
    return memory


def make_uf2(memory):
    addresses = sorted({address & ~255 for address in memory})
    blocks = []
    for index, address in enumerate(addresses):
        header = struct.pack(
            "<8I", 0x0A324655, 0x9E5D5157, 0x2000, address, 256, index, len(addresses), UF2_FAMILY
        )
        payload = bytes(memory.get(address + i, 255) for i in range(256))
        blocks.append(header + payload + bytes(220) + struct.pack("<I", 0x0AB16F30))
    return b"".join(blocks)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--objcopy", default="arm-none-eabi-objcopy")
    parser.add_argument("--target-dir", type=Path, default=ROOT / "build" / "target")
    parser.add_argument("--out", type=Path, default=ROOT / "build" / "uf2")
    parser.add_argument("--package-only", action="store_true", help="use previously built ELF files")
    args = parser.parse_args()
    target = args.target_dir.resolve()
    env = {**os.environ, "CARGO_TARGET_DIR": str(target)}
    jobs = [
        ("0x5aca212f", [("left", "LEFT"), ("right", "RIGHT")]),
        ("0x60ebba71", [("dongle", "RECEIVER")]),
    ]
    args.out.mkdir(parents=True, exist_ok=True)
    for build_hash, roles in jobs:
        if not args.package_only:
            bins = [argument for name, _ in roles for argument in ("--bin", name)]
            subprocess.run(
                ["cargo", "build", "--release", "--locked", *bins],
                cwd=ROOT / "firmware", env={**env, "PANDA_BUILD_HASH": build_hash}, check=True,
            )
        for binary, role in roles:
            image = target / "thumbv7em-none-eabihf" / "release" / binary
            hex_file = args.out / f"{binary}.hex"
            subprocess.run([args.objcopy, "-O", "ihex", image, hex_file], check=True)
            memory = load_hex(hex_file)
            data = bytes(memory.get(address, 255) for address in range(APP_START, max(memory) + 1))
            if struct.pack("<I", int(build_hash, 16)) not in data:
                raise ValueError(f"Unexpected storage build hash for {role}")
            if role != "RECEIVER" and b"UI RC8" not in data:
                raise ValueError(f"Unexpected display version for {role}")
            uf2 = make_uf2(memory)
            out = args.out / f"PandaKB_Sofle_{role}_UI_RC8.uf2"
            out.write_bytes(uf2)
            print(f"{hashlib.sha256(uf2).hexdigest()}  {out}")


if __name__ == "__main__":
    main()

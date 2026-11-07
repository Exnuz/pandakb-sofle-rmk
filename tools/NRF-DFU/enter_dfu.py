"""Enter the existing PandaKB LEFT USB UF2 bootloader; never proxy through a receiver."""
from pathlib import Path
import argparse
import ctypes
import os
import sys
import time

VID, LEFT_PID, RECEIVER_PID = 0x4C4B, 0x5350, 0x5351
ROLES={0x5350:(1,"left"),0x5352:(2,"right"),0x5351:(3,"receiver")}
UID = bytes.fromhex('50534c524d4b5332')
HERE = Path(__file__).resolve().parent

def devices(hid):
    return [d for d in hid.enumerate(VID, 0)
            if d.get('usage_page') == (0xFF61 if d['product_id']==0x5352 else 0xFF60) and d.get('usage') == 0x61]

def select_device(found, serial=None, role='auto'):
    eligible=[d for d in found if d['product_id'] in ROLES and (serial is None or d.get('serial_number')==serial) and (role=='auto' or ROLES[d['product_id']][1]==role)]
    if len(eligible)>1:
        raise ValueError('Several boards found: use -Role left/right/receiver or -Serial. No reset sent.')
    if not eligible:
        raise ValueError('Requested board has no USB maintenance interface. Connect directly by a data cable; RIGHT needs RC4 installed once using double Reset.')
    return eligible[0]

def packet(command):
    if len(command) > 32:
        raise ValueError('Vial payload is limited to 32 bytes')
    return b'\x00' + bytes(command).ljust(32, b'\x00')

def query(handle, command):
    if handle.write(packet(command)) != 33:
        raise OSError('Incomplete HID write')
    response = bytes(handle.read(32, 1800))
    if len(response) != 32:
        raise OSError('No full Vial reply. Close Vial and retry.')
    return response

def reboot(hid, selected):
    handle = hid.device()
    try:
        handle.open_path(selected['path'])
        # Drop queued replies before issuing our own read-only identification queries.
        for _ in range(16):
            if not handle.read(32, 1):
                break
        role=ROLES[selected['product_id']][0]
        info=bytes([0xfc,80,78,82,70,1,0])
        response=query(handle,info)
        if response[:6]==info[:6] and response[6]==0:
            if response[7]!=role:
                raise ValueError('Local role mismatch: refusing a forwarded reset.')
            command=bytes([0xfc,80,78,82,70,1,1,role])+b'BOOT'
        elif role==1:
            response=query(handle,[0xfe,0])
            if response[4:12]!=UID:
                raise ValueError('Unexpected keyboard ID. Reset NOT sent.')
            command=bytes([0x0b])
        else:
            raise ValueError('This board needs RC4 installed once with double Reset. Reset NOT sent to another board.')
        if handle.write(packet(command))!=33:
            raise OSError('Boot write incomplete; check Explorer for NICENANO.')
    finally:
        handle.close()

def boot_drives():
    if os.name != 'nt':
        return {}
    kernel = ctypes.windll.kernel32
    mask = kernel.GetLogicalDrives()
    found = {}
    for n in range(26):
        if not mask & (1 << n):
            continue
        root = f'{chr(65+n)}:\\'
        if kernel.GetDriveTypeW(ctypes.c_wchar_p(root)) not in (2, 3):
            continue
        try:
            info = (Path(root) / 'INFO_UF2.TXT').read_text(errors='replace')[:4096]
            if 'nrf52840' in info.lower() or 'nicenano' in info.lower() or 'nice!nano' in info.lower():
                found[root] = info
        except OSError:
            pass
    return found

def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--list', action='store_true', help='list USB interfaces without resetting')
    parser.add_argument('--dry-run', action='store_true', help='select target without sending any HID command')
    parser.add_argument('--role',choices=['auto','left','right','receiver'],default='auto')
    parser.add_argument('--serial', help='select one LEFT board by its exact USB serial number')
    args = parser.parse_args(argv)
    sys.path.insert(0, str(HERE / 'lib'))
    import hid
    found = devices(hid)
    if args.list:
        for d in found:
            role=ROLES.get(d['product_id'],(0,'unknown'))[1]
            print(f"{d['vendor_id']:04X}:{d['product_id']:04X} {role}; serial={d.get('serial_number')!r}")
        if not found:
            print('No PandaKB Vial USB interfaces found.')
        return 0
    before = boot_drives()
    try:
        target = select_device(found, args.serial,args.role)
    except ValueError:
        if not found and before and not args.serial:
            print('An nRF UF2 bootloader is already available: ' + ', '.join(before))
            return 0
        raise
    print(f"Target: {ROLES[target['product_id']][1]}, direct USB, {VID:04X}:{target['product_id']:04X}, serial={target.get('serial_number')!r}")
    if args.dry_run:
        print('DRY RUN: no HID commands sent, no reset performed.')
        return 0
    print('Requesting UF2 bootloader...', flush=True)
    reboot(hid, target)
    print('Boot command sent. Waiting for a new nRF UF2 drive...', flush=True)
    deadline = time.monotonic() + 12
    while time.monotonic() < deadline:
        new = boot_drives().keys() - before.keys()
        if new:
            print('UF2 bootloader drive detected: ' + ', '.join(sorted(new)))
            print('Copy the matching role .uf2 firmware to that drive. Nothing was flashed by this script.')
            return 0
        time.sleep(0.25)
    print('Command sent, but a new UF2 drive was not confirmed within 12 seconds. '
          'Check Explorer for NICENANO. If absent, double-press Reset.')
    return 2

if __name__ == '__main__':
    try:
        raise SystemExit(main())
    except (OSError, ValueError, ImportError) as exc:
        print('ERROR: ' + str(exc), file=sys.stderr)
        raise SystemExit(1)

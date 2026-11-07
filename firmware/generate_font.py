from pathlib import Path
import json
p=Path(__file__).resolve().parent
d=json.loads((p/'pixel-font.json').read_text())
s='// Pixel glyphs used in the approved OLED preview.\npub fn glyph(c:char)->(i16,[u8;5]) {match c {\n'
for k,v in d.items():
    s+=repr(k)+'=>('+str(len(v[0]))+','+str([int(x,2) for x in v])+'),\n'
s+='_ => (2,[0;5])}}\n'
(p/'src/pixel_font.rs').write_text(s)

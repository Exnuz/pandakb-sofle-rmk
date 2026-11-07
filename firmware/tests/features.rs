use super::*;
use rmk_types::action::{Action,KeyAction,LightAction};
fn packet(payload:&[u8])->[u8;32]{let mut req=[0u8;32];req[..payload.len()].copy_from_slice(payload);let mut reply=req;panda_lighting::packet(&req,&mut reply);reply}
pub fn verify(){
 // Use the real Vial keycode decoder AND encoder. A saved RGB key must read back unchanged.
 for code in (0x7800..=0x7806).chain(0x7820..=0x782e){let decoded=keycode_convert::from_via_keycode(code);assert!(matches!(decoded,KeyAction::Single(Action::Light(_))));assert_eq!(keycode_convert::to_via_keycode(decoded),code);}
 let info=packet(&[8,0x40]);assert_eq!(&info[2..5],&[1,0,255]);
 let mut after=0u16;let mut supported=vec![0];
 loop{let req=[8,0x42,after as u8,(after>>8)as u8];let reply=packet(&req);let mut end=false;for pair in reply[2..].chunks_exact(2){let mode=u16::from_le_bytes([pair[0],pair[1]]);if mode==65535{end=true;break;}assert!(mode>after);after=mode;supported.push(mode);}if end{break;}}
 assert_eq!(supported,vec![0,2,3,6,13,14]);
 for (vial,internal) in [(0u16,4),(2,5),(3,0),(6,6),(13,7),(14,2)]{
  let reply=packet(&[7,0x41,vial as u8,(vial>>8)as u8,179,85,201,153]);assert_eq!(reply[0],7);assert_eq!(menu::RGB.load(Relaxed),internal);assert_eq!(menu::BRIGHT.load(Relaxed),60);
  let reply=packet(&[8,0x41]);assert_eq!(u16::from_le_bytes([reply[2],reply[3]]),vial);assert_eq!(&reply[4..8],&[179,85,201,153]);
 }
 let before=menu::RGB.load(Relaxed);assert_eq!(packet(&[7,0x41,0xff,0xff])[0],255);assert_eq!(menu::RGB.load(Relaxed),before);assert_eq!(packet(&[9])[0],255,"Save must not falsely report persistence");
 menu::RGB.store(2,Relaxed);panda_lighting::action(LightAction::RgbTog);assert_eq!(menu::RGB.load(Relaxed),4);panda_lighting::action(LightAction::RgbTog);assert_eq!(menu::RGB.load(Relaxed),2);
 menu::BRIGHT.store(100,Relaxed);panda_lighting::action(LightAction::RgbVai);assert_eq!(menu::BRIGHT.load(Relaxed),100);for _ in 0..30{panda_lighting::action(LightAction::RgbVad);}assert_eq!(menu::BRIGHT.load(Relaxed),0);
 // Exactly ten distinct keyboard-controlled saturation levels, both endpoints included.
 menu::RGB.store(5,Relaxed);panda_lighting::SAT.store(0,Relaxed);
 for expected in panda_lighting::SAT_LEVELS.iter().skip(1){panda_lighting::action(LightAction::RgbSai);assert_eq!(panda_lighting::SAT.load(Relaxed),*expected);}
 panda_lighting::action(LightAction::RgbSai);assert_eq!(panda_lighting::SAT.load(Relaxed),255);
 for expected in panda_lighting::SAT_LEVELS.iter().rev().skip(1){panda_lighting::action(LightAction::RgbSad);assert_eq!(panda_lighting::SAT.load(Relaxed),*expected);}
 panda_lighting::action(LightAction::RgbSad);assert_eq!(panda_lighting::SAT.load(Relaxed),0);
 for value in 0..=255{let up=panda_lighting::saturation_step(value,true);let down=panda_lighting::saturation_step(value,false);assert!(panda_lighting::SAT_LEVELS.contains(&up)&&panda_lighting::SAT_LEVELS.contains(&down));assert!(up>=value&&down<=value);if value<255{assert!(up>value);}if value>0{assert!(down<value);}}
 // WHITE reports zero saturation even when a previous HSV setting was saturated.
 menu::RGB.store(3,Relaxed);panda_lighting::SAT.store(255,Relaxed);panda_lighting::action(LightAction::RgbSai);assert_eq!(panda_lighting::SAT.load(Relaxed),28);
 println!("PASS saturation: exactly 10 levels, bidirectional clamping, all 256 starting values, WHITE starts from zero saturation");
 menu::BRIGHT.store(0, Relaxed);
 for expected in panda_lighting::BRIGHTNESS_LEVELS.iter().skip(1) {
     panda_lighting::action(LightAction::RgbVai);
     assert_eq!(menu::BRIGHT.load(Relaxed), *expected);
 }
 for expected in panda_lighting::BRIGHTNESS_LEVELS.iter().rev().skip(1) {
     block_on(menu::command(23));
     assert_eq!(menu::BRIGHT.load(Relaxed), *expected);
 }
 menu::BRIGHT.store(20, Relaxed);
 block_on(menu::command(22));
 assert_eq!(menu::BRIGHT.load(Relaxed), 22);
 menu::BRIGHT.store(20, Relaxed);
 panda_lighting::action(LightAction::BacklightDown);
 assert_eq!(menu::BRIGHT.load(Relaxed), 11);

 use rmk_types::{ble::{BleState, BleStatus}, connection::{ConnectionStatus, ConnectionType, UsbState}};
 let mut status = ConnectionStatus {
     preferred: ConnectionType::Ble,
     usb: UsbState::Enabled,
     ble: BleStatus { profile: 3, state: BleState::Advertising },
 };
 assert_eq!(render::connection_badge(status, true), (0, 1));
 status.usb = UsbState::Configured;
 assert_eq!(render::connection_badge(status, true), (4, 2));
 assert_eq!(render::connection_badge(status, false), (0, 0));
 status.usb = UsbState::Suspended;
 assert_eq!(render::connection_badge(status, true), (4, 2));
 status.ble.state = BleState::Connected;
 assert_eq!(render::connection_badge(status, true), (0, 2));
 status.usb = UsbState::Disabled;
 status.ble.state = BleState::Advertising;
 assert_eq!(render::connection_badge(status, true), (0, 1));

 let mut palette = panda_colors::RemotePalette::default();
 let current = || panda_colors::REMOTE.each_ref().map(|byte| byte.load(Relaxed));
 let old = current();
 for (offset, value) in [(0, 1), (9, 2), (27, 4), (18, 3), (27, 4)] {
     palette.receive(offset, [value; 27]);
     assert_eq!(current(), old, "An incomplete palette must not leak into the rendered frame");
 }
 for (offset, value) in [(0, 5), (9, 6), (18, 7)] {
     palette.receive(offset, [value; 27]);
     assert_eq!(current(), old);
 }
 palette.receive(27, [8; 27]);
 for (chunk, expected) in current().chunks_exact(27).zip(5..=8) {
     assert!(chunk.iter().all(|value| *value == expected));
 }

 use rmk_types::keycode::{HidKeyCode, KeyCode};
 for code in 0xCD..=0xDC {
     let action = keycode_convert::from_via_keycode(code);
     assert!(matches!(action, KeyAction::Single(Action::Key(KeyCode::Hid(key))) if key.is_mouse_key()));
     assert_eq!(keycode_convert::to_via_keycode(action), code);
 }
 for (code, layer, tap) in [(0x462C, 6, HidKeyCode::Space), (0x472A, 7, HidKeyCode::Backspace)] {
     let action = keycode_convert::from_via_keycode(code);
     assert!(matches!(action, KeyAction::TapHold(Action::Key(KeyCode::Hid(key)), Action::LayerOn(id), _) if key == tap && id == layer));
     assert_eq!(keycode_convert::to_via_keycode(action), code);
 }
 println!("PASS RC7: ten brightness positions, active-transport USB badge, complete remote palettes, Vial mouse and layer-tap codes");
 panda_lighting::HUE.store(0,Relaxed);panda_lighting::SAT.store(255,Relaxed);assert_eq!(panda_lighting::color(5,100,0,0,0),[48,0,0]);panda_lighting::SAT.store(0,Relaxed);assert_eq!(panda_lighting::color(5,100,0,0,0),[48,48,48]);
 assert_eq!(panda_lighting::color(0,100,0,0,0),[0;3]);assert_ne!(panda_lighting::color(0,100,0,0,8),[0;3]);
 for hue in 0..=255{panda_lighting::HUE.store(hue,Relaxed);for mode in 0..8{for index in 0..36{assert!(panda_lighting::color(mode,100,7,u64::MAX,index).iter().all(|v|*v<=48));assert_eq!(panda_lighting::color(mode,0,7,u64::MAX,index),[0;3]);}}}
 let mut wpm=panda_wpm::Estimator::new();assert_eq!(wpm.sample(0),0);assert!(wpm.sample(1)>0,"Typing must appear on the next 250 ms sample");for _ in 0..8{wpm.sample(5);}assert_eq!(wpm.sample(5),240,"Five keys per 250 ms is 240 WPM");for _ in 0..8{wpm.sample(0);}assert_eq!(wpm.sample(0),0,"Idle window must decay fully to zero");
 let mut matrix=panda_matrix::MatrixState::new(10,6);
 for row in 0..10{for col in 0..6{matrix.update(&key(row,col,true));assert!(matrix.read(row,col));let mut report=[0;30];matrix.read_all(&mut report);assert_eq!(report[row as usize],1<<col);assert_eq!(report.iter().map(|v|v.count_ones()).sum::<u32>(),1);matrix.update(&key(row,col,false));assert!(!matrix.read(row,col));}}
 // Unlock uses physical positions, including menu-consumed buttons.
 matrix.update(&key(0,0,true));matrix.update(&key(4,4,true));assert!(matrix.read(0,0)&&matrix.read(4,4));matrix.update(&key(4,5,true));assert!(matrix.read(4,5));matrix.update(&key(4,5,false));assert!(!matrix.read(4,5));
 println!("PASS: actual RGB keycodes round-trip; VialRGB discovery/list/set/get; unsupported Save rejected; HSV/brightness and electrical cap; WPM 250 ms response/two-second window; all 60 physical matrix press/release bits and unlock positions");
}

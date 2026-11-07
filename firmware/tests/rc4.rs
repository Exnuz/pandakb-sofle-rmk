use super::*;
pub fn verify(){
 for role in 1..=3{let mut req=[0;32];req[..6].copy_from_slice(&[0xfc,b'P',b'N',b'R',b'F',1]);let mut reply=[0;32];assert_eq!(panda_maintenance::packet(role,&req,&mut reply),Some(false));assert_eq!(reply[7],role);req[6]=1;req[7]=role;req[8..12].copy_from_slice(b"BOOT");assert_eq!(panda_maintenance::packet(role,&req,&mut reply),Some(true));req[7]=0;assert_eq!(panda_maintenance::packet(role,&req,&mut reply),Some(false));}
 let mut unlock=[255;32];unlock[..6].copy_from_slice(&[0,0,0,0,4,4]);let mut matrix_req=[0;32];matrix_req[..2].copy_from_slice(&[2,3]);let mut status_req=[0;32];status_req[..2].copy_from_slice(&[0xfe,5]);
 assert!(!panda_reply::matches(&matrix_req,&unlock),"Screenshot's phantom pattern is an unlock reply, never a matrix");assert!(panda_reply::matches(&status_req,&unlock));let mut matrix=[0;32];matrix[..2].copy_from_slice(&[2,3]);assert!(panda_reply::matches(&matrix_req,&matrix));assert!(!panda_reply::matches(&status_req,&matrix));
 for layer in 0..8{for offset in [0u8,9,18,27,36,45,54,63]{let mut req=[0;32];req[..5].copy_from_slice(&[2,0xa5,layer,offset,9]);for i in 0..27{req[5+i]=(i+offset as usize)as u8;}let mut reply=[0;32];assert!(panda_colors::packet(&req,&mut reply));req[1]=0xa7;assert!(panda_colors::packet(&req,&mut reply));assert_eq!(&req[5..],&reply[5..]);}}
 let mut req=[0;32];req[..5].copy_from_slice(&[2,0xa5,8,0,1]);let mut reply=[0;32];assert!(panda_colors::packet(&req,&mut reply));assert_eq!(reply[0],255);
 req[..5].copy_from_slice(&[2,0xa8,0,36,1]);req[5]=2;panda_colors::packet(&req,&mut reply);menu::LEDS.store(0,Relaxed);assert_eq!(panda_colors::color(0,36,0),[0;3]);menu::LEDS.store(2,Relaxed);assert_ne!(panda_colors::color(0,36,0),[0;3]);
 req[..2].copy_from_slice(&[2,0xa4]);panda_colors::packet(&req,&mut reply);assert_eq!(menu::RGB.load(Relaxed),8);
 println!("PASS RC4: three local USB DFU roles and wrong-target rejection; unlock/matrix response correlation reproduces screenshot; all 8 x 72 per-key colors; boundaries and Caps gate");
}

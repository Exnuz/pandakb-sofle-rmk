//! Local USB identity/reset, deliberately distinct from forwarded Vial commands.
pub fn packet(role:u8,req:&[u8;32],reply:&mut[u8;32])->Option<bool>{
 if req[..6]!=[0xfc,b'P',b'N',b'R',b'F',1]{return None;}
 *reply=*req;reply[7]=role;
 match req[6]{0=>Some(false),1 if req[7]==role&&req[8..12]==*b"BOOT"=>Some(true),_=>{reply[0]=255;Some(false)}}
}

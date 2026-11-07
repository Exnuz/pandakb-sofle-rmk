//! Correlate self-identifying Vial replies; never decode unlock bytes as a matrix.
pub fn matches(req:&[u8;32],reply:&[u8;32])->bool{
 match (req[0],req[1]){
  (2,3)=>reply[0]==2&&reply[1]==3,
  (0xfe,5)=>reply[0]<=1&&reply[1]<=1&&reply[2..6]==[0,0,4,4]&&reply[6..].iter().all(|v|*v==255),
  (1,_)=>reply[0]==1,
  (0x11,_)=>reply[0]==0x11,
  (7|8,_)=>reply[0]==req[0]&&reply[1]==req[1],
  (2,0xa4..=0xab)=>reply[0]==2&&reply[1]==req[1]||reply[0]==255&&reply[1]==req[1],
  _=>true
 }
}

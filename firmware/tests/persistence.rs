use super::*;
use embedded_storage::nor_flash::{ErrorType,NorFlashError,NorFlashErrorKind};
use embedded_storage_async::nor_flash::{ReadNorFlash,NorFlash};
#[derive(Debug)]struct Failed;
impl NorFlashError for Failed{fn kind(&self)->NorFlashErrorKind{NorFlashErrorKind::Other}}
struct Memory{bytes:Vec<u8>,writes:u8,erases:u8,fail:Option<u8>}
impl ErrorType for Memory{type Error=Failed;}
impl ReadNorFlash for Memory{const READ_SIZE:usize=1;async fn read(&mut self,offset:u32,out:&mut[u8])->Result<(),Failed>{out.copy_from_slice(&self.bytes[offset as usize..offset as usize+out.len()]);Ok(())}fn capacity(&self)->usize{8192}}
impl NorFlash for Memory{const WRITE_SIZE:usize=4;const ERASE_SIZE:usize=4096;
 async fn erase(&mut self,start:u32,end:u32)->Result<(),Failed>{assert_eq!(start%4096,0);assert_eq!(end-start,4096);self.bytes[start as usize..end as usize].fill(255);self.erases+=1;Ok(())}
 async fn write(&mut self,offset:u32,data:&[u8])->Result<(),Failed>{assert_eq!(offset%4,0);assert_eq!(data.len()%4,0);self.writes+=1;if self.fail==Some(self.writes){return Err(Failed);}for(i,b)in data.iter().enumerate(){let old=&mut self.bytes[offset as usize+i];assert_eq!(*old&b,*b);*old=*b;}Ok(())}
}
pub fn verify(){
 let lighting = panda_record::LIGHTING;
 let overrides = panda_record::Format::new(*b"PKO1", panda_override::PAYLOAD_BYTES);
 for (format, payload) in [(lighting, panda_record::PAYLOAD), (overrides, panda_override::PAYLOAD_BYTES)] {
 for cut in [1,2] {
  let mut flash=Memory{bytes:vec![255;8192],writes:0,erases:0,fail:None};let mut a=[255;4096];let mut b=[255;4096];
  assert!(block_on(format.commit(&mut flash,0,&mut a,&mut b,|p|p[16..16+payload].fill(17))));
  assert_eq!(format.valid(flash.bytes[..4096].try_into().unwrap()),Some(1));
  if payload == panda_record::PAYLOAD {
   assert_eq!(&flash.bytes[..16], b"PC04\x01\x00\x00\x00\xc6\x0f\x00\x00\xfc\x09\xff\x42");
   assert!(overrides.valid(flash.bytes[..4096].try_into().unwrap()).is_none());
  } else {
   assert!(lighting.valid(flash.bytes[..4096].try_into().unwrap()).is_none());
  }
  flash.writes=0;flash.fail=Some(cut);
  assert!(!block_on(format.commit(&mut flash,0,&mut a,&mut b,|p|p[16..16+payload].fill(27))));
  assert_eq!(format.valid(flash.bytes[..4096].try_into().unwrap()),Some(1),"Last confirmed page survives lost power before commit");assert_eq!(format.valid(flash.bytes[4096..].try_into().unwrap()),None);
  flash.fail=None;assert!(block_on(format.commit(&mut flash,0,&mut a,&mut b,|p|p[16..16+payload].fill(27))));
  assert_eq!(format.valid(flash.bytes[4096..].try_into().unwrap()),Some(2));let erases=flash.erases;
  assert!(block_on(format.commit(&mut flash,0,&mut a,&mut b,|p|p[16..16+payload].fill(27))));assert_eq!(flash.erases,erases,"Duplicate Save must not wear flash");
  flash.bytes[4096+40]^=1;assert_eq!(format.valid(flash.bytes[4096..].try_into().unwrap()),None,"CRC rejects corruption");
 }
 }
 assert!(panda_record::newer(0,u32::MAX));
 println!("PASS persistence: real async dual-page commit, power loss before body/commit, read-back confirmation, duplicate Save without erase, CRC corruption and sequence wrap");
}

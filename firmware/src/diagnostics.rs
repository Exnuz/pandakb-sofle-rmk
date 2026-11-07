//! Hardware diagnostics start before RMK initializes the radio.
use core::{convert::Infallible, fmt::Write, sync::atomic::{AtomicU8, AtomicBool, Ordering}};
use embassy_nrf::{bind_interrupts, gpio::{Output, Level, OutputDrive}, peripherals::*, Peri, spim, twim};
use embassy_time::{Duration, Timer};
use embedded_graphics::{prelude::*, pixelcolor::BinaryColor, mono_font::{ascii::FONT_5X7, MonoTextStyle}, text::Text};
use rmk::types::{ble::BleState, connection::{ConnectionType, UsbState}};

static OLED_INIT: AtomicBool = AtomicBool::new(false);
pub static PHASE: AtomicU8 = AtomicU8::new(1);
bind_interrupts!(struct DiagIrqs {
    TWISPI0 => twim::InterruptHandler<TWISPI0>;
    SPIM3 => spim::InterruptHandler<SPI3>;
});

struct Frame([u8; 512]);
impl OriginDimensions for Frame { fn size(&self) -> Size { Size::new(128,32) } }
impl DrawTarget for Frame {
    type Color=BinaryColor;
    type Error=Infallible;
    fn draw_iter<I: IntoIterator<Item=Pixel<BinaryColor>>>(&mut self, pixels:I)->Result<(),Infallible> {
        for Pixel(p,c) in pixels { if p.x>=0 && p.x<128 && p.y>=0 && p.y<32 {
            let n=p.x as usize+(p.y as usize/8)*128;
            if c==BinaryColor::On { self.0[n]|=1<<(p.y%8); }
        }} Ok(())
    }
}
fn command(i2c:&mut twim::Twim<'static>, bytes:&[u8]) -> bool {
    let mut b=[0u8;32]; b[1..bytes.len()+1].copy_from_slice(bytes);
    i2c.blocking_write_timeout(0x3c,&b[..bytes.len()+1],Duration::from_millis(15)).is_ok()
}
fn screen(i2c:&mut twim::Twim<'static>, lines:[&str;4]) -> bool {
    // Initialize once, retry only after an I2C error.
    if !OLED_INIT.load(Ordering::Relaxed) {
    if !command(i2c,&[0xAE,0xD5,0x80,0xA8,31,0xD3,0,0x40,0x8D,0x14,0x20,0,0xA1,0xC8,0xDA,2,0x81,0x5F,0xD9,0xF1,0xDB,0x40,0xA4,0xA6,0xAF]) {return false;}
    OLED_INIT.store(true,Ordering::Relaxed);
    }
    command(i2c,&[0x21,0,127,0x22,0,3]);
    let mut f=Frame([0;512]);
    for (n,line) in lines.iter().enumerate() {
        Text::new(line,Point::new(0,7+8*n as i32),MonoTextStyle::new(&FONT_5X7,BinaryColor::On)).draw(&mut f).ok();
    }
    for chunk in f.0.chunks(16) {
        let mut b=[0x40u8;17]; b[1..].copy_from_slice(chunk);
        if i2c.blocking_write_timeout(0x3c,&b,Duration::from_millis(15)).is_err() {OLED_INIT.store(false,Ordering::Relaxed);return false;}
    } true
}
fn white(spi:&mut spim::Spim<'static>) {
    // Same 4MHz encoding as the working PandaKB ZMK shield, 36 LEDs.
    // Dim white: RGB 4/255, keeping startup current small.
    let mut data=[0u8;36*24+64];
    for led in 0..36 { for channel in 0..3 { for bit in 0..8 {
        data[led*24+channel*8+bit]=if 4u8 & (0x80>>bit)!=0 {0x70} else {0x40};
    }}}
    spi.blocking_write(&data).ok();
}
pub async fn start(spawner:embassy_executor::Spawner, bus:Peri<'static,TWISPI0>, spi:Peri<'static,SPI3>,sda:Peri<'static,P0_17>,scl:Peri<'static,P0_20>,mosi:Peri<'static,P0_06>,power:Peri<'static,P0_13>) {
    let power=Output::new(power,Level::High,OutputDrive::Standard);
    Timer::after_millis(50).await; // nice_nano_v2 EXT_POWER init delay
    let i2c=twim::Twim::new(bus,DiagIrqs,sda,scl,twim::Config::default(),&mut []);
    let mut cfg=spim::Config::default(); cfg.frequency=spim::Frequency::M4;
    let spi=spim::Spim::new_txonly_nosck(spi,DiagIrqs,mosi,cfg);
    spawner.spawn(task(i2c,spi,power).unwrap()); if env!("CARGO_BIN_NAME")=="left" { spawner.spawn(auto_dongle().unwrap()); }
    Timer::after_millis(400).await;
}
#[embassy_executor::task]
async fn task(mut i2c:twim::Twim<'static>,mut spi:spim::Spim<'static>,_power:Output<'static>) {
    let role=if env!("CARGO_BIN_NAME")=="left" {"LEFT TEST6"} else {"RIGHT TEST6"};
    loop {
        white(&mut spi);
        let stage=PHASE.load(Ordering::Relaxed);
        let st=rmk::diagnostic_connection();
        let mut line=heapless::String::<32>::new();
        let mut transport=heapless::String::<32>::new();
        let state=match st.ble.state {BleState::Advertising=>"PAIR",BleState::Connected=>"LINK",BleState::Inactive=>"OFF"};
        let mode=if st.ble.profile==3 {"DONGLE"} else {"BT"};
        write!(line,"{}{} {}",mode,st.ble.profile,state).ok();
        let usb=match st.usb {UsbState::Configured=>"OK",UsbState::Suspended=>"SLEEP",UsbState::Enabled=>"ENUM",UsbState::Disabled=>"OFF"};
        let pref=match st.preferred {ConnectionType::Usb=>"USB",ConnectionType::Ble=>"BLE"};
        write!(transport,"USB:{} PREF:{}",usb,pref).ok();
        let mut title=heapless::String::<32>::new();
        write!(title,"{} U{}",role,embassy_time::Instant::now().as_secs()).ok();
        let count=rmk::DIAG_KEY_COUNT.load(Ordering::Relaxed);
        let key=rmk::DIAG_LAST_KEY.load(Ordering::Relaxed);
        let mut keys=heapless::String::<32>::new();
        if key==0xffffffff {write!(keys,"S:{} K0 --",if rmk::DIAGNOSTIC_SPLIT.load(Ordering::Relaxed){"Y"}else{"N"}).ok();}
        else {write!(keys,"S:{} K{} R{}C{} {}",if rmk::DIAGNOSTIC_SPLIT.load(Ordering::Relaxed){"Y"}else{"N"},count,(key>>16)&255,(key>>8)&255,if key&1!=0{"DN"}else{"UP"}).ok();}
        if stage<3 { screen(&mut i2c,[&title,if stage==1 {"BOOT: RADIO"} else {"BOOT: SDC"},"RGB: WHITE","WAIT..."]); }
        else {screen(&mut i2c,[&title,&transport,&line,&keys]);}
        Timer::after_millis(500).await;
    }
}




#[embassy_executor::task]
async fn auto_dongle() {
    Timer::after_secs(5).await;
    rmk::diagnostic_select_dongle().await;
}


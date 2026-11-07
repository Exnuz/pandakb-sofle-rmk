//! WPM (Words Per Minute) processor for RMK
use core::sync::atomic::{AtomicU16, Ordering};

use rmk_macro::processor;

use crate::event::{KeyboardEvent, KeyboardEventPos, WpmUpdateEvent, publish_event};


/// Latest WPM, written alongside every `WpmUpdateEvent` publish so host
/// services can read the current value synchronously without subscribing.
static CURRENT_WPM: AtomicU16 = AtomicU16::new(0);

/// Current typing speed in words per minute. Returns `0` when no
/// `WpmProcessor` is running.
pub(crate) fn current_wpm() -> u16 {
    CURRENT_WPM.load(Ordering::Relaxed)
}

/// Processor to estimate typing speed in words per minute (WPM)
#[processor(subscribe = [KeyboardEvent], poll_interval = 250)]
pub struct WpmProcessor {
    keys_pressed: u8,
    wpm: u16,
    estimator: crate::panda_wpm::Estimator,
}

impl Default for WpmProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl WpmProcessor {
    pub fn new() -> Self {
        Self {
            keys_pressed: 0,
            wpm: 0,
            estimator: crate::panda_wpm::Estimator::new(),
        }
    }

    async fn on_keyboard_event(&mut self, event: KeyboardEvent) {
        if let KeyboardEvent { pressed: false, pos:KeyboardEventPos::Key(k) } = event {
            if crate::panda_ui::PAGE.load(Ordering::Relaxed)==0 && !(k.col==5&&(k.row==4||k.row==9)) {
                self.keys_pressed = self.keys_pressed.saturating_add(1);
            }
        }
    }

    async fn poll(&mut self) {
        let avg_wpm = self.estimator.sample(self.keys_pressed);

        if avg_wpm != self.wpm {
            self.wpm = avg_wpm;
            CURRENT_WPM.store(self.wpm, Ordering::Relaxed);
            publish_event(WpmUpdateEvent::new(self.wpm));
        }

        self.keys_pressed = 0;
    }
}

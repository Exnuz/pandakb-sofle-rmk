//! Vial's dynamic key override wire format and keyboard report processing.
use rmk_types::{keycode::HidKeyCode, modifier::ModifierCombination};

pub const COUNT: usize = 8;
pub const RULE_BYTES: usize = 10;
pub const PAYLOAD_BYTES: usize = COUNT * RULE_BYTES;
const TRIGGER_DOWN: u8 = 1 << 0;
const MOD_DOWN: u8 = 1 << 1;
const NEGATIVE_UP: u8 = 1 << 2;
const ANY_MOD: u8 = 1 << 3;
const NO_RESTORE: u8 = 1 << 4;
const KEEP_ON_OTHER_PRESS: u8 = 1 << 5;
const ENABLED: u8 = 1 << 7;
const ACTIVATIONS: u8 = TRIGGER_DOWN | MOD_DOWN | NEGATIVE_UP;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Rule {
    pub trigger: u16,
    pub replacement: u16,
    pub layers: u16,
    pub required: u8,
    pub negative: u8,
    pub suppressed: u8,
    pub options: u8,
}

impl Rule {
    pub const fn decode(bytes: [u8; RULE_BYTES]) -> Self {
        let [t0, t1, r0, r1, l0, l1, required, negative, suppressed, options] = bytes;
        Self {
            trigger: u16::from_le_bytes([t0, t1]),
            replacement: u16::from_le_bytes([r0, r1]),
            layers: u16::from_le_bytes([l0, l1]),
            required, negative, suppressed, options,
        }
    }

    pub fn encode(self) -> [u8; RULE_BYTES] {
        let [t0, t1] = self.trigger.to_le_bytes();
        let [r0, r1] = self.replacement.to_le_bytes();
        let [l0, l1] = self.layers.to_le_bytes();
        [t0, t1, r0, r1, l0, l1, self.required, self.negative, self.suppressed, self.options]
    }

    pub fn supported(self) -> bool {
        self.options & ENABLED == 0
            || (self.trigger <= 0xFF
                && keyboard_code(self.trigger as u8).is_some_and(|key| !key.is_modifier())
                && self.replacement <= 0x1FFF
                && keyboard_code(self.replacement as u8).is_some())
    }

    fn matches_modifiers(self, mods: u8) -> bool {
        if self.negative & mods != 0 {
            return false;
        }
        let active = self.required & mods;
        if self.required != 0 && self.options & ANY_MOD != 0 {
            active != 0
        } else {
            // When both sides of one modifier are allowed, either side suffices.
            (active | active >> 4) & 0x0F == (self.required | self.required >> 4) & 0x0F
        }
    }

    fn activates_on(self, event: u8) -> bool {
        let mask = self.options & ACTIVATIONS;
        let allowed = if mask == 0 { ACTIVATIONS } else { mask };
        event & allowed != 0
    }

    fn output(self) -> (u8, u8) {
        let mut mods = ModifierCombination::from_packed_bits((self.replacement >> 8) as u8).into_bits();
        let code = self.replacement as u8;
        if let Some(key) = HidKeyCode::from_repr(code).filter(|key| key.is_modifier()) {
            mods |= key.to_hid_modifiers().into_bits();
            (0, mods)
        } else {
            (code, mods)
        }
    }
}

fn keyboard_code(code: u8) -> Option<HidKeyCode> {
    HidKeyCode::from_repr(code).filter(|key| {
        !matches!(code, 1..=3)
            && !key.is_mouse_key()
            && key.process_as_consumer().is_none()
            && key.process_as_system_control().is_none()
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Trigger {
    slot: usize,
    code: u8,
    layer: u8,
    since: u64,
}

impl Trigger {
    fn repeat_at(self, now: u64) -> u64 {
        let repeat = self.since + 500;
        if repeat > now { repeat } else { now + 50 }
    }
}

#[derive(Clone, Copy, Debug)]
struct Active {
    rule: Rule,
    trigger: Option<Trigger>,
    ready_at: Option<u64>,
}

#[derive(Default)]
pub struct State {
    last: Option<Trigger>,
    new_press: bool,
    active: Option<Active>,
    restore: Option<(Trigger, u64)>,
    hidden: u8,
    previous_mods: u8,
}

impl State {
    pub fn press(&mut self, slot: usize, code: u8, layer: u8, now: u64) {
        self.release(slot);
        self.last = Some(Trigger { slot, code, layer, since: now });
        self.new_press = true;
        self.restore = None;
    }

    pub fn release(&mut self, slot: usize) {
        self.hidden &= !(1 << slot);
        if self.last.is_some_and(|key| key.slot == slot) {
            self.last = None;
            self.new_press = false;
        }
        if self.active.and_then(|active| active.trigger).is_some_and(|key| key.slot == slot) {
            self.active = None;
        }
        if self.restore.is_some_and(|(key, _)| key.slot == slot) {
            self.restore = None;
        }
    }

    pub fn next_deadline(&self) -> Option<u64> {
        self.active.and_then(|active| active.ready_at).or(self.restore.map(|(_, at)| at))
    }

    pub fn report(
        &mut self, rules: &[Rule; COUNT], mut codes: [u8; 6], mut mods: u8,
        match_mods: u8, current_layer: u8, now: u64,
    ) -> ([u8; 6], u8) {
        let pressed = core::mem::take(&mut self.new_press);
        let down = match_mods & !self.previous_mods;
        let up = self.previous_mods & !match_mods;
        self.previous_mods = match_mods;

        if let Some(active) = self.active {
            if !active.rule.matches_modifiers(match_mods) {
                self.active = None;
                if active.rule.options & NO_RESTORE == 0
                    && let Some(trigger) = active.trigger
                    && Some(trigger) == self.last
                {
                    self.restore = Some((trigger, trigger.repeat_at(now)));
                }
            } else if pressed && active.rule.options & KEEP_ON_OTHER_PRESS == 0 {
                self.active = None;
            }
        }

        if pressed || down != 0 || up != 0 {
            for &rule in rules {
                if rule.options & ENABLED == 0
                    || self.active.is_some_and(|active| active.rule == rule)
                    || !rule.matches_modifiers(match_mods)
                {
                    continue;
                }
                let trigger = if rule.trigger == 0 {
                    None
                } else if let Some(key) = self.last.filter(|key| u16::from(key.code) == rule.trigger) {
                    Some(key)
                } else {
                    continue;
                };
                let layer = trigger.map_or(current_layer, |key| key.layer);
                if layer >= 16 || rule.layers & (1 << layer) == 0 {
                    continue;
                }
                let trigger_press = pressed && trigger.is_some();
                let event = if trigger_press {
                    TRIGGER_DOWN
                } else {
                    (if down & rule.required != 0 { MOD_DOWN } else { 0 })
                        | (if up & rule.negative != 0 { NEGATIVE_UP } else { 0 })
                };
                if !rule.activates_on(event) {
                    continue;
                }
                self.restore = None;
                if let Some(key) = trigger {
                    self.hidden |= 1 << key.slot;
                }
                let ready_at = if trigger_press { None } else { Some(trigger.map_or(now + 50, |key| key.repeat_at(now))) };
                self.active = Some(Active { rule, trigger, ready_at });
                break;
            }
        }

        if let Some((trigger, at)) = self.restore && now >= at {
            self.hidden &= !(1 << trigger.slot);
            self.restore = None;
        }
        for (slot, code) in codes.iter_mut().enumerate() {
            if self.hidden & (1 << slot) != 0 {
                *code = 0;
            }
        }
        if let Some(active) = &mut self.active {
            mods &= !active.rule.suppressed;
            if active.ready_at.is_none_or(|at| now >= at) {
                active.ready_at = None;
                let (code, extra_mods) = active.rule.output();
                mods |= extra_mods;
                if let Some(target) = active.trigger.map(|key| key.slot).or_else(|| codes.iter().position(|&key| key == 0)) {
                    codes[target] = code;
                }
            }
        }
        (codes, mods)
    }
}

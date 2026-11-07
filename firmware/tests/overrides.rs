use super::panda_override::{COUNT, Rule, State};

const F1: u8 = 0x3A;
const F2: u8 = 0x3B;
const A: u8 = 4;
const LEFT_SHIFT: u8 = 2;
const RIGHT_SHIFT: u8 = 32;
const CONTROL: u8 = 1;
const SHIFT: u8 = LEFT_SHIFT | RIGHT_SHIFT;
const EMPTY: [u8; 6] = [0; 6];

fn keys(a: u8, b: u8) -> [u8; 6] {
    [a, b, 0, 0, 0, 0]
}

fn table(rule: Rule) -> [Rule; COUNT] {
    let mut rules = [Rule::default(); COUNT];
    rules[0] = rule;
    rules
}

pub fn verify() {
    // Python Vial struct.pack("<HHHBBBB", 0x3A, 0x3B, 1, 0x22, 1, 0x22, 0x87).
    let packet = [0x3A, 0, 0x3B, 0, 1, 0, 0x22, 1, 0x22, 0x87];
    let rule = Rule::decode(packet);
    assert_eq!(rule.encode(), packet);
    assert!(rule.supported());
    assert!(!Rule { replacement: 0x7C00, ..rule }.supported());
    let rules = table(rule);

    for (mods, layer, expected) in [
        (0, 0, (F1, 0)),
        (LEFT_SHIFT, 0, (F2, 0)),
        (RIGHT_SHIFT, 0, (F2, 0)),
        (SHIFT, 0, (F2, 0)),
        (LEFT_SHIFT | CONTROL, 0, (F1, LEFT_SHIFT | CONTROL)),
        (LEFT_SHIFT, 1, (F1, LEFT_SHIFT)),
    ] {
        let mut state = State::default();
        state.press(0, F1, layer, 10);
        assert_eq!(state.report(&rules, keys(F1, 0), mods, mods, 0, 10), (keys(expected.0, 0), expected.1));
        state.release(0);
        assert_eq!(state.report(&rules, EMPTY, mods, mods, 0, 20), (EMPTY, mods));
    }

    // If a new key interrupts the override, it keeps the real Shift state.
    let mut state = State::default();
    state.press(0, F1, 0, 10);
    assert_eq!(state.report(&rules, keys(F1, 0), SHIFT, SHIFT, 0, 10), (keys(F2, 0), 0));
    state.press(1, A, 0, 20);
    assert_eq!(state.report(&rules, keys(F1, A), SHIFT, SHIFT, 0, 20), (keys(0, A), SHIFT));
    state.release(1);
    assert_eq!(state.report(&rules, keys(F1, 0), SHIFT, SHIFT, 0, 25), (EMPTY, SHIFT));
    state.release(0);
    assert_eq!(state.report(&rules, EMPTY, SHIFT, SHIFT, 0, 30), (EMPTY, SHIFT));

    // Modifier changes while holding the trigger defer the second character.
    let mut state = State::default();
    state.press(0, F1, 0, 10);
    assert_eq!(state.report(&rules, keys(F1, 0), 0, 0, 0, 10), (keys(F1, 0), 0));
    assert_eq!(state.report(&rules, keys(F1, 0), SHIFT, SHIFT, 0, 20), (EMPTY, 0));
    assert_eq!(state.next_deadline(), Some(510));
    assert_eq!(state.report(&rules, keys(F1, 0), SHIFT, SHIFT, 0, 510), (keys(F2, 0), 0));
    assert_eq!(state.next_deadline(), None);
    assert_eq!(state.report(&rules, keys(F1, 0), 0, 0, 0, 520), (EMPTY, 0));
    assert_eq!(state.next_deadline(), Some(570));
    assert_eq!(state.report(&rules, keys(F1, 0), 0, 0, 0, 570), (keys(F1, 0), 0));

    let trigger_only = table(Rule { options: 0x81, ..rule });
    let mut state = State::default();
    state.press(0, F1, 0, 10);
    assert_eq!(state.report(&trigger_only, keys(F1, 0), 0, 0, 0, 10), (keys(F1, 0), 0));
    assert_eq!(state.report(&trigger_only, keys(F1, 0), SHIFT, SHIFT, 0, 20), (keys(F1, 0), SHIFT));

    let no_restore = table(Rule { options: 0x91, ..rule });
    let mut state = State::default();
    state.press(0, F1, 0, 10);
    assert_eq!(state.report(&no_restore, keys(F1, 0), SHIFT, SHIFT, 0, 10), (keys(F2, 0), 0));
    assert_eq!(state.report(&no_restore, keys(F1, 0), 0, 0, 0, 20), (EMPTY, 0));
    assert_eq!(state.next_deadline(), None);

    let keep = table(Rule { options: 0xA1, replacement: 0x0104, ..rule });
    let mut state = State::default();
    state.press(0, F1, 0, 10);
    assert_eq!(state.report(&keep, keys(F1, 0), SHIFT, SHIFT, 0, 10), (keys(A, 0), CONTROL));
    state.press(1, 5, 0, 20);
    assert_eq!(state.report(&keep, keys(F1, 5), SHIFT, SHIFT, 0, 20), (keys(A, 5), CONTROL));

    let modifier_only = table(Rule { trigger: 0, ..rule });
    let mut state = State::default();
    assert_eq!(state.report(&modifier_only, EMPTY, SHIFT, SHIFT, 0, 1), (EMPTY, 0));
    assert_eq!(state.report(&modifier_only, EMPTY, SHIFT, SHIFT, 0, 51), (keys(F2, 0), 0));
    assert_eq!(state.report(&modifier_only, EMPTY, 0, 0, 0, 70), (EMPTY, 0));

    let any_mod = table(Rule { required: CONTROL | SHIFT, negative: 0, options: 0x89, ..rule });
    let mut state = State::default();
    state.press(0, F1, 0, 10);
    assert_eq!(state.report(&any_mod, keys(F1, 0), CONTROL, CONTROL, 0, 10), (keys(F2, 0), CONTROL));

    println!("PASS RC8: Vial override bytes, either Shift, layer mask, forbidden mods, release order, activation flags and modified replacements");
}

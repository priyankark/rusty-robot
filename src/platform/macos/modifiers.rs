//! Track physical modifier keys separately; Quartz event flags group both sides.
const KEYS: [(u32, u64); 8] = [
    (56, 1 << 17),
    (60, 1 << 17), // left/right Shift
    (59, 1 << 18),
    (62, 1 << 18), // left/right Control
    (58, 1 << 19),
    (61, 1 << 19), // left/right Option
    (55, 1 << 20),
    (54, 1 << 20), // left/right Command
];

pub(super) fn transition(mut held: u8, code: u32, down: bool) -> (u8, u64) {
    if let Some(index) = KEYS.iter().position(|(key, _)| *key == code) {
        if down {
            held |= 1 << index;
        } else {
            held &= !(1 << index);
        }
    }
    let flags = KEYS
        .iter()
        .enumerate()
        .filter(|(index, _)| held & (1 << index) != 0)
        .fold(0, |flags, (_, (_, mask))| flags | mask);
    (held, flags)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn releasing_either_side_preserves_the_other_side() {
        for pair in KEYS.as_chunks::<2>().0 {
            for order in [[pair[0], pair[1]], [pair[1], pair[0]]] {
                let (held, _) = transition(0, order[0].0, true);
                let (held, flags) = transition(held, order[1].0, true);
                assert_eq!(flags, order[0].1);
                let (held, flags) = transition(held, order[0].0, false);
                assert_eq!(flags, order[1].1);
                assert_eq!(transition(held, order[1].0, false), (0, 0));
            }
        }
    }
    #[test]
    fn repeated_down_and_unheld_opposite_release_are_idempotent() {
        for pair in KEYS.as_chunks::<2>().0 {
            for order in [[pair[0], pair[1]], [pair[1], pair[0]]] {
                let state = transition(0, order[0].0, true);
                assert_eq!(transition(state.0, order[0].0, true), state);
                assert_eq!(transition(state.0, order[1].0, false), state);
                assert_eq!(transition(state.0, order[0].0, false), (0, 0));
            }
        }
    }
    #[test]
    fn ordinary_keys_preserve_every_modifier_combination() {
        for held in 0..=u8::MAX {
            let expected = transition(held, 0, true);
            assert_eq!(expected.0, held);
            for code in [0, 36, 48, 123, 126] {
                assert_eq!(transition(held, code, true), expected);
                assert_eq!(transition(held, code, false), expected);
            }
        }
    }
    #[test]
    fn modifier_families_do_not_clear_each_other() {
        let mut held = 0;
        for (code, _) in KEYS {
            held = transition(held, code, true).0;
        }
        assert_eq!(held, u8::MAX);
        let all = (1 << 17) | (1 << 18) | (1 << 19) | (1 << 20);
        assert_eq!(transition(held, 0, true).1, all);
        for pair in KEYS.as_chunks::<2>().0 {
            let (state, flags) = transition(held, pair[0].0, false);
            assert_eq!(flags, all);
            assert_eq!(transition(state, pair[1].0, false).1, all & !pair[0].1);
        }
    }
}

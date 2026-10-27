use super::*;
use std::collections::BTreeSet;

#[derive(Debug, PartialEq)]
enum Event {
    Key(u32, bool, u32),
    Text(char),
}
#[derive(Default)]
struct Recorder {
    events: Vec<Event>,
    held: BTreeSet<u32>,
    fail_events: Vec<usize>,
    fail_lookup: Option<&'static str>,
}
impl KeyboardOutput for Recorder {
    fn key_code(&mut self, key: &str) -> Result<u32> {
        if self.fail_lookup == Some(key) {
            return Err(err("lookup failed"));
        }
        Ok(match key {
            "command" => 8,
            "alt" => 4,
            "control" => 2,
            "shift" => 1,
            "enter" => 13,
            _ => panic!("unexpected key {key}"),
        })
    }
    fn key_event(&mut self, code: u32, down: bool, flags: u32) -> Result<()> {
        self.events.push(Event::Key(code, down, flags));
        if self.fail_events.contains(&self.events.len()) {
            return Err(err(format!("event {} failed", self.events.len())));
        }
        if down {
            self.held.insert(code);
        } else {
            self.held.remove(&code);
        }
        Ok(())
    }
    fn unicode(&mut self, character: char) -> Result<()> {
        self.events.push(Event::Text(character));
        if self.fail_events.contains(&self.events.len()) {
            return Err(err("unicode failed"));
        }
        Ok(())
    }
}

#[test]
fn newline_and_carriage_return_use_complete_native_return_presses() {
    for ch in ['\n', '\r'] {
        let mut recorder = Recorder::default();
        type_character(&mut recorder, ch).unwrap();
        assert_eq!(
            recorder.events,
            [Event::Key(13, true, 0), Event::Key(13, false, 0)]
        );
        assert!(recorder.held.is_empty());
    }
}
#[test]
fn unicode_scalars_are_preserved_including_combining_and_zwj_sequences() {
    let text = "café e\u{301} 中文 مرحبا 👩‍💻 🚀\0";
    let mut recorder = Recorder::default();
    for ch in text.chars() {
        type_character(&mut recorder, ch).unwrap();
    }
    assert_eq!(
        recorder.events,
        text.chars().map(Event::Text).collect::<Vec<_>>()
    );
}
#[test]
fn unicode_delivery_failure_is_reported() {
    let mut recorder = Recorder {
        fail_events: vec![1],
        ..Default::default()
    };
    assert_eq!(
        type_character(&mut recorder, '🚀').unwrap_err().to_string(),
        "unicode failed"
    );
}
#[test]
fn return_key_failure_is_reported_without_extra_presses() {
    let mut recorder = Recorder {
        fail_events: vec![1],
        ..Default::default()
    };
    assert!(type_character(&mut recorder, '\n').is_err());
    assert_eq!(recorder.events.len(), 1);
    assert!(recorder.held.is_empty());
}
#[test]
fn every_modifier_combination_is_held_for_the_key_and_released() {
    for flags in 0..16 {
        let mut recorder = Recorder::default();
        toggle(&mut recorder, 42, true, flags).unwrap();
        assert!(recorder.events.contains(&Event::Key(42, true, flags)));
        let expected: BTreeSet<_> = [1, 2, 4, 8, 42]
            .into_iter()
            .filter(|c| *c == 42 || flags & c != 0)
            .collect();
        assert_eq!(recorder.held, expected, "flags={flags}");
        toggle(&mut recorder, 42, false, flags).unwrap();
        assert!(recorder.held.is_empty(), "flags={flags}");
        assert_eq!(
            recorder.events.last(),
            Some(&Event::Key(
                if flags == 0 { 42 } else { 1 << flags.ilog2() },
                false,
                0
            ))
        );
    }
}
#[test]
fn failure_at_each_press_releases_previously_pressed_modifiers() {
    for failure in 1..=5 {
        let mut recorder = Recorder {
            fail_events: vec![failure],
            ..Default::default()
        };
        let error = toggle(&mut recorder, 42, true, 15).unwrap_err();
        assert_eq!(error.to_string(), format!("event {failure} failed"));
        assert!(
            recorder.held.is_empty(),
            "failure={failure}: {:?}",
            recorder.events
        );
        if failure > 1 {
            assert_eq!(
                recorder.events.last(),
                Some(&Event::Key(8, false, 0)),
                "cleanup must not advertise the modifier whose press failed"
            );
        }
    }
}
#[test]
fn modifier_lookup_failure_does_not_leave_a_modifier_held() {
    for key in ["command", "alt", "control", "shift"] {
        let mut recorder = Recorder {
            fail_lookup: Some(key),
            ..Default::default()
        };
        assert!(toggle(&mut recorder, 42, true, 15).is_err());
        assert!(
            recorder.events.is_empty(),
            "{key} lookup failed after events: {:?}",
            recorder.events
        );
        assert!(recorder.held.is_empty());
    }
}

#[test]
fn failure_at_each_release_does_not_prevent_other_releases() {
    for (failure, stuck) in [(6, 42), (7, 1), (8, 2), (9, 4), (10, 8)] {
        let mut recorder = Recorder::default();
        toggle(&mut recorder, 42, true, 15).unwrap();
        recorder.fail_events = vec![failure];
        assert_eq!(
            toggle(&mut recorder, 42, false, 15)
                .unwrap_err()
                .to_string(),
            format!("event {failure} failed")
        );
        assert_eq!(recorder.held, BTreeSet::from([stuck]));
        assert_eq!(recorder.events.len(), 10);
    }
}

#[test]
fn return_release_failure_is_not_reported_as_success() {
    let mut recorder = Recorder {
        fail_events: vec![2],
        ..Default::default()
    };
    assert_eq!(
        type_character(&mut recorder, '\n').unwrap_err().to_string(),
        "event 2 failed"
    );
    assert_eq!(recorder.events.len(), 2);
}
#[test]
fn key_release_failure_still_attempts_all_modifier_releases() {
    let mut recorder = Recorder::default();
    toggle(&mut recorder, 42, true, 15).unwrap();
    recorder.fail_events = vec![6];
    assert_eq!(
        toggle(&mut recorder, 42, false, 15)
            .unwrap_err()
            .to_string(),
        "event 6 failed"
    );
    // The transport rejected release of the main key; modifiers must still clear.
    assert_eq!(recorder.held, BTreeSet::from([42]));
    assert_eq!(
        &recorder.events[6..],
        &[
            Event::Key(1, false, 14),
            Event::Key(2, false, 12),
            Event::Key(4, false, 8),
            Event::Key(8, false, 0),
        ]
    );
}
#[test]
fn cleanup_continues_after_a_release_failure_and_preserves_original_error() {
    let mut recorder = Recorder {
        fail_events: vec![5, 6],
        ..Default::default()
    };
    assert_eq!(
        toggle(&mut recorder, 42, true, 15).unwrap_err().to_string(),
        "event 5 failed"
    );
    assert_eq!(recorder.held, BTreeSet::from([1]));
    assert_eq!(recorder.events.len(), 9);
}
#[test]
fn aliases_and_duplicates_do_not_duplicate_modifiers() {
    assert_eq!(
        modifiers(&["none", "shift", "right_shift", "shift"]).unwrap(),
        1
    );
    assert_eq!(
        modifiers(&[
            "control",
            "left_control",
            "right_control",
            "alt",
            "right_alt",
            "command"
        ])
        .unwrap(),
        14
    );
    assert!(modifiers(&["shift", "invalid"]).is_err());
}
#[test]
fn unicode_interval_keeps_platform_floor_and_larger_requested_delays() {
    let minimum = if cfg!(target_os = "macos") {
        Duration::from_millis(10)
    } else {
        Duration::ZERO
    };
    assert_eq!(text_interval(Duration::ZERO), minimum);
    assert_eq!(
        text_interval(Duration::from_nanos(1)),
        minimum.max(Duration::from_nanos(1))
    );
    for delay in [
        Duration::from_millis(10),
        Duration::from_millis(50),
        Duration::from_millis(u64::MAX),
    ] {
        assert_eq!(text_interval(delay), delay);
    }
}

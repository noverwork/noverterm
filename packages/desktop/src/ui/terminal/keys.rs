//! Turns key presses into the bytes a terminal program expects: xterm's legacy
//! encoding, or the Kitty keyboard protocol once the program asked for it.

use alacritty_terminal::term::TermMode;

/// One key event, independent of the UI toolkit.
#[derive(Debug, Clone, Copy, Default)]
pub struct Key<'a> {
    /// Unshifted key name: `a`, `1`, `enter`, `up`, `f5`, ...
    pub key: &'a str,
    /// Text the key types with its modifiers applied, if any.
    pub text: Option<&'a str>,
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
    /// Cmd on macOS, Super/Windows elsewhere.
    pub platform: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEvent {
    Press,
    Repeat,
    Release,
}

/// Bytes to send for `key`, or `None` when the key is plain text that the
/// input method should deliver instead (or means nothing to a terminal).
///
/// `alt_is_meta` is false on macOS, where Option composes characters.
pub fn encode(key: &Key, event: KeyEvent, mode: TermMode, alt_is_meta: bool) -> Option<Vec<u8>> {
    if mode.intersects(TermMode::KITTY_KEYBOARD_PROTOCOL) {
        return kitty(key, event, mode, alt_is_meta);
    }
    if event == KeyEvent::Release {
        return None;
    }
    legacy(key, mode, alt_is_meta).map(String::into_bytes)
}

fn modifier_param(key: &Key) -> u8 {
    1 + u8::from(key.shift)
        + 2 * u8::from(key.alt)
        + 4 * u8::from(key.ctrl)
        + 8 * u8::from(key.platform)
}

/// The final byte and number of keys that xterm writes as `CSI 1;mods X` or
/// `CSI n;mods ~`.
enum Functional {
    Letter(char),
    Tilde(u8),
}

fn functional(name: &str) -> Option<Functional> {
    Some(match name {
        "up" => Functional::Letter('A'),
        "down" => Functional::Letter('B'),
        "right" => Functional::Letter('C'),
        "left" => Functional::Letter('D'),
        "home" => Functional::Letter('H'),
        "end" => Functional::Letter('F'),
        "f1" => Functional::Letter('P'),
        "f2" => Functional::Letter('Q'),
        "f3" => Functional::Letter('R'),
        "f4" => Functional::Letter('S'),
        "insert" => Functional::Tilde(2),
        "delete" => Functional::Tilde(3),
        "pageup" => Functional::Tilde(5),
        "pagedown" => Functional::Tilde(6),
        "f5" => Functional::Tilde(15),
        "f6" => Functional::Tilde(17),
        "f7" => Functional::Tilde(18),
        "f8" => Functional::Tilde(19),
        "f9" => Functional::Tilde(20),
        "f10" => Functional::Tilde(21),
        "f11" => Functional::Tilde(23),
        "f12" => Functional::Tilde(24),
        _ => return None,
    })
}

fn legacy(key: &Key, mode: TermMode, alt_is_meta: bool) -> Option<String> {
    let mods = modifier_param(key);
    let esc_if_alt = |text: &str| {
        if key.alt {
            format!("\x1b{text}")
        } else {
            text.to_string()
        }
    };

    // xterm only reports Ctrl on PageUp/PageDown, and leaves Shift/Ctrl+Insert
    // to the clipboard.
    let mods = match key.key {
        "pageup" | "pagedown" if !key.ctrl => 1,
        _ => mods,
    };
    if key.key == "insert" && (key.shift || key.ctrl) {
        return None;
    }

    if let Some(functional) = functional(key.key) {
        return Some(match functional {
            Functional::Letter(letter) if mods == 1 => {
                let function_key = matches!(letter, 'P'..='S');
                let cursor_key = !function_key && mode.contains(TermMode::APP_CURSOR);
                if function_key || cursor_key {
                    format!("\x1bO{letter}")
                } else {
                    format!("\x1b[{letter}")
                }
            }
            Functional::Letter(letter) => format!("\x1b[1;{mods}{letter}"),
            Functional::Tilde(number) if mods == 1 => format!("\x1b[{number}~"),
            Functional::Tilde(number) => format!("\x1b[{number};{mods}~"),
        });
    }

    match key.key {
        // Shift+Enter is indistinguishable from Enter in the legacy encoding;
        // CSI u lets editors and agent CLIs tell them apart.
        "enter" if key.shift && !key.ctrl && !key.alt => return Some("\x1b[13;2u".to_string()),
        "enter" => return Some(esc_if_alt("\r")),
        "tab" if key.shift => return Some("\x1b[Z".to_string()),
        "tab" => return Some(esc_if_alt("\t")),
        "backspace" if key.ctrl => return Some(esc_if_alt("\x08")),
        "backspace" => return Some(esc_if_alt("\x7f")),
        "escape" => return Some(esc_if_alt("\x1b")),
        "space" if key.ctrl => return Some(esc_if_alt("\0")),
        "space" if key.alt && alt_is_meta => return Some("\x1b ".to_string()),
        _ => {}
    }

    let mut chars = key.key.chars();
    let (Some(character), None) = (chars.next(), chars.next()) else {
        return None;
    };
    if key.ctrl {
        let control = match character.to_ascii_lowercase() {
            letter @ 'a'..='z' => letter as u8 - b'a' + 1,
            '@' | '2' => 0,
            '[' | '3' => 0x1b,
            '\\' | '4' => 0x1c,
            ']' | '5' => 0x1d,
            '^' | '6' => 0x1e,
            '_' | '/' | '-' | '7' => 0x1f,
            '?' | '8' => 0x7f,
            _ => return None,
        };
        return Some(esc_if_alt(&char::from(control).to_string()));
    }
    if key.alt && alt_is_meta {
        let text = key
            .text
            .map_or_else(|| character.to_string(), str::to_string);
        return Some(format!("\x1b{text}"));
    }
    // Plain text: the input method delivers it, composed or not.
    None
}

/// Kitty's number for a key that types no text.
fn kitty_functional_code(name: &str) -> Option<u32> {
    Some(match name {
        "escape" => 27,
        "enter" => 13,
        "tab" => 9,
        "backspace" => 127,
        "space" => 32,
        _ => return None,
    })
}

/// US-layout symbols and the key that types them with Shift. The toolkit
/// reports Shift+= as an unshifted "+", which the protocol wants as "=" + Shift.
const SHIFTED_SYMBOLS: &str = "~!@#$%^&*()_+{}|:\"<>?";
const UNSHIFTED_SYMBOLS: &str = "`1234567890-=[]\\;',./";

fn kitty(key: &Key, event: KeyEvent, mode: TermMode, alt_is_meta: bool) -> Option<Vec<u8>> {
    let mut key = *key;
    if !key.shift && key.key.chars().count() == 1 {
        if let Some(index) = SHIFTED_SYMBOLS.find(key.key) {
            let base = &UNSHIFTED_SYMBOLS[index..index + 1];
            key.text = key.text.or(Some(key.key));
            key.key = base;
            key.shift = true;
        }
    }
    let key = &key;
    // Option types characters on macOS; the composed text arrives as input.
    let printable = key.key.chars().count() == 1;
    if key.alt && !alt_is_meta && !key.ctrl && !key.platform && printable {
        return None;
    }
    let report_events = mode.contains(TermMode::REPORT_EVENT_TYPES);
    let report_all = mode.contains(TermMode::REPORT_ALL_KEYS_AS_ESC);
    if event == KeyEvent::Release && !report_events {
        return None;
    }
    let mods = modifier_param(key);
    let event_suffix = match event {
        KeyEvent::Press => "",
        KeyEvent::Repeat if report_events => ":2",
        KeyEvent::Repeat => "",
        KeyEvent::Release => ":3",
    };
    let modifiers = if mods == 1 && event_suffix.is_empty() {
        String::new()
    } else {
        format!(";{mods}{event_suffix}")
    };

    if let Some(functional) = functional(key.key) {
        return Some(
            match functional {
                // Kitty moved F3 off `CSI R`, which collides with cursor reports.
                Functional::Letter('R') => format!("\x1b[13{modifiers}~"),
                Functional::Letter(letter) if modifiers.is_empty() => {
                    if mode.contains(TermMode::APP_CURSOR) && !matches!(letter, 'P'..='S') {
                        format!("\x1bO{letter}")
                    } else {
                        format!("\x1b[{letter}")
                    }
                }
                Functional::Letter(letter) => format!("\x1b[1{modifiers}{letter}"),
                Functional::Tilde(number) => format!("\x1b[{number}{modifiers}~"),
            }
            .into_bytes(),
        );
    }

    let named = kitty_functional_code(key.key);
    let mut chars = key.key.chars();
    let single = match (chars.next(), chars.next()) {
        (Some(character), None) => Some(character),
        _ => None,
    };
    let code = named.or(single.map(|character| character.to_ascii_lowercase() as u32))?;

    // Enter, Tab and Backspace keep their legacy bytes until a program asks
    // for every key as an escape code; Escape is what "disambiguate" is for.
    let legacy_named = matches!(key.key, "enter" | "tab" | "backspace");
    let has_real_modifier = key.ctrl || key.alt || key.platform;
    let plain = !has_real_modifier && (named.is_none() || legacy_named || key.key == "space");
    if plain && !report_all {
        if event == KeyEvent::Release {
            return None;
        }
        return match key.key {
            "enter" if key.shift => Some(b"\x1b[13;2u".to_vec()),
            "enter" => Some(b"\r".to_vec()),
            "tab" if key.shift => Some(b"\x1b[9;2u".to_vec()),
            "tab" => Some(b"\t".to_vec()),
            "backspace" => Some(b"\x7f".to_vec()),
            // Text keys go through the input method.
            _ => None,
        };
    }

    let mut sequence = format!("\x1b[{code}");
    if mode.contains(TermMode::REPORT_ALTERNATE_KEYS) && key.shift {
        if let Some(shifted) = key.text.and_then(|text| text.chars().next()) {
            if shifted as u32 != code {
                sequence.push_str(&format!(":{}", shifted as u32));
            }
        }
    }
    let text = key
        .text
        .filter(|_| mode.contains(TermMode::REPORT_ASSOCIATED_TEXT))
        .filter(|_| event != KeyEvent::Release && !key.ctrl && !key.platform)
        .filter(|text| !text.chars().any(char::is_control));
    match text {
        Some(text) => {
            let codepoints: Vec<String> = text
                .chars()
                .map(|character| (character as u32).to_string())
                .collect();
            sequence.push_str(&format!(";{mods}{event_suffix};{}", codepoints.join(":")));
        }
        None => sequence.push_str(&modifiers),
    }
    sequence.push('u');
    Some(sequence.into_bytes())
}

/// Text the input method committed (typed, composed or dictated). Once a
/// program asked for every key as an escape code, text is reported as one too.
pub fn text(text: &str, mode: TermMode) -> Vec<u8> {
    if !mode.contains(TermMode::REPORT_ALL_KEYS_AS_ESC) || text.chars().any(char::is_control) {
        return text.as_bytes().to_vec();
    }
    if !mode.contains(TermMode::REPORT_ASSOCIATED_TEXT) {
        return Vec::new();
    }
    let codepoints: Vec<String> = text.chars().map(|c| (c as u32).to_string()).collect();
    format!("\x1b[0;;{}u", codepoints.join(":")).into_bytes()
}

/// A modifier key going down or up, which only "report all keys" programs see.
pub fn modifier(name: &str, pressed: bool, key: &Key, mode: TermMode) -> Option<Vec<u8>> {
    if !mode.contains(TermMode::REPORT_ALL_KEYS_AS_ESC)
        || (!pressed && !mode.contains(TermMode::REPORT_EVENT_TYPES))
    {
        return None;
    }
    let code = match name {
        "shift" => 57441,
        "control" => 57442,
        "alt" => 57443,
        "platform" => 57444,
        _ => return None,
    };
    let event = if pressed { "" } else { ":3" };
    Some(format!("\x1b[{code};{}{event}u", modifier_param(key)).into_bytes())
}

/// Wraps pasted text for the program on the other end. Line breaks become CR,
/// as a typed Enter would send.
pub fn paste(text: &str, mode: TermMode) -> Vec<u8> {
    let text = text.replace("\r\n", "\r").replace('\n', "\r");
    if mode.contains(TermMode::BRACKETED_PASTE) {
        // A pasted "end of paste" marker would let the clipboard run commands.
        let safe = text.replace('\x1b', "");
        format!("\x1b[200~{safe}\x1b[201~").into_bytes()
    } else {
        text.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str) -> Key<'_> {
        Key {
            key: name,
            ..Key::default()
        }
    }

    fn sent(key: Key, mode: TermMode) -> Option<String> {
        encode(&key, KeyEvent::Press, mode, true).map(|bytes| String::from_utf8(bytes).unwrap())
    }

    #[test]
    fn legacy_keys_match_xterm() {
        let none = TermMode::NONE;
        assert_eq!(sent(key("a"), none), None);
        assert_eq!(
            sent(
                Key {
                    ctrl: true,
                    ..key("c")
                },
                none
            )
            .as_deref(),
            Some("\x03")
        );
        assert_eq!(
            sent(
                Key {
                    alt: true,
                    ..key("b")
                },
                none
            )
            .as_deref(),
            Some("\x1bb")
        );
        assert_eq!(sent(key("up"), none).as_deref(), Some("\x1b[A"));
        assert_eq!(
            sent(key("up"), TermMode::APP_CURSOR).as_deref(),
            Some("\x1bOA")
        );
        assert_eq!(
            sent(
                Key {
                    ctrl: true,
                    ..key("right")
                },
                none
            )
            .as_deref(),
            Some("\x1b[1;5C")
        );
        assert_eq!(sent(key("f1"), none).as_deref(), Some("\x1bOP"));
        assert_eq!(
            sent(
                Key {
                    shift: true,
                    ..key("f5")
                },
                none
            )
            .as_deref(),
            Some("\x1b[15;2~")
        );
        assert_eq!(sent(key("delete"), none).as_deref(), Some("\x1b[3~"));
        assert_eq!(
            sent(
                Key {
                    shift: true,
                    ..key("tab")
                },
                none
            )
            .as_deref(),
            Some("\x1b[Z")
        );
        assert_eq!(
            sent(
                Key {
                    shift: true,
                    ..key("enter")
                },
                none
            )
            .as_deref(),
            Some("\x1b[13;2u")
        );
        assert_eq!(sent(key("backspace"), none).as_deref(), Some("\x7f"));
        // Option composes on macOS instead of sending ESC.
        let option = Key {
            alt: true,
            ..key("b")
        };
        assert_eq!(encode(&option, KeyEvent::Press, none, false), None);
        assert_eq!(encode(&key("a"), KeyEvent::Release, none, true), None);
    }

    #[test]
    fn kitty_disambiguates_and_reports() {
        let disambiguate = TermMode::DISAMBIGUATE_ESC_CODES;
        assert_eq!(
            sent(key("escape"), disambiguate).as_deref(),
            Some("\x1b[27u")
        );
        assert_eq!(sent(key("a"), disambiguate), None);
        assert_eq!(sent(key("enter"), disambiguate).as_deref(), Some("\r"));
        assert_eq!(
            sent(
                Key {
                    ctrl: true,
                    ..key("c")
                },
                disambiguate
            )
            .as_deref(),
            Some("\x1b[99;5u")
        );
        assert_eq!(
            sent(
                Key {
                    ctrl: true,
                    ..key("up")
                },
                disambiguate
            )
            .as_deref(),
            Some("\x1b[1;5A")
        );
        assert_eq!(sent(key("f3"), disambiguate).as_deref(), Some("\x1b[13~"));

        let events = disambiguate | TermMode::REPORT_EVENT_TYPES;
        let release = encode(
            &Key {
                ctrl: true,
                ..key("c")
            },
            KeyEvent::Release,
            events,
            true,
        );
        assert_eq!(release.as_deref(), Some(&b"\x1b[99;5:3u"[..]));
        assert_eq!(
            encode(&key("a"), KeyEvent::Release, disambiguate, true),
            None
        );

        let all =
            disambiguate | TermMode::REPORT_ALL_KEYS_AS_ESC | TermMode::REPORT_ASSOCIATED_TEXT;
        let shifted = Key {
            shift: true,
            text: Some("A"),
            ..key("a")
        };
        assert_eq!(sent(shifted, all).as_deref(), Some("\x1b[97;2;65u"));
        assert_eq!(sent(key("enter"), all).as_deref(), Some("\x1b[13u"));
    }

    #[test]
    fn paste_brackets_and_strips_escape() {
        assert_eq!(paste("a\nb", TermMode::NONE), b"a\rb");
        assert_eq!(
            paste("x\x1b[201~y\r\nz", TermMode::BRACKETED_PASTE),
            b"\x1b[200~x[201~y\rz\x1b[201~"
        );
    }

    #[test]
    fn legacy_matches_xterm_edge_keys() {
        let none = TermMode::NONE;
        let alt = |name| Key {
            alt: true,
            ..key(name)
        };
        assert_eq!(sent(alt("enter"), none).as_deref(), Some("\x1b\r"));
        assert_eq!(sent(alt("escape"), none).as_deref(), Some("\x1b\x1b"));
        assert_eq!(sent(alt("backspace"), none).as_deref(), Some("\x1b\x7f"));
        assert_eq!(sent(alt("space"), none).as_deref(), Some("\x1b "));
        assert_eq!(sent(alt("pageup"), none).as_deref(), Some("\x1b[5~"));
        assert_eq!(
            sent(
                Key {
                    ctrl: true,
                    ..key("pagedown")
                },
                none
            )
            .as_deref(),
            Some("\x1b[6;5~")
        );
        assert_eq!(
            sent(
                Key {
                    shift: true,
                    ..key("insert")
                },
                none
            ),
            None
        );
        assert_eq!(
            sent(
                Key {
                    ctrl: true,
                    ..key("/")
                },
                none
            )
            .as_deref(),
            Some("\x1f")
        );
    }

    #[test]
    fn kitty_matches_the_previous_front_end() {
        let flags = |bits: u8| {
            let mut mode = TermMode::NONE;
            for (bit, flag) in [
                (1, TermMode::DISAMBIGUATE_ESC_CODES),
                (2, TermMode::REPORT_EVENT_TYPES),
                (4, TermMode::REPORT_ALTERNATE_KEYS),
                (8, TermMode::REPORT_ALL_KEYS_AS_ESC),
                (16, TermMode::REPORT_ASSOCIATED_TEXT),
            ] {
                if bits & bit != 0 {
                    mode |= flag;
                }
            }
            mode
        };
        let send = |key: Key, event, mode| {
            encode(&key, event, mode, false).map(|bytes| String::from_utf8(bytes).unwrap())
        };
        // Alt+Enter is what agent CLIs read as "newline".
        let alt_enter = Key {
            alt: true,
            ..key("enter")
        };
        assert_eq!(
            send(alt_enter, KeyEvent::Press, flags(5)).as_deref(),
            Some("\x1b[13;3u")
        );
        let ctrl_shift_enter = Key {
            ctrl: true,
            shift: true,
            ..key("enter")
        };
        assert_eq!(
            send(ctrl_shift_enter, KeyEvent::Press, flags(1)).as_deref(),
            Some("\x1b[13;6u")
        );
        // Option still composes on macOS.
        let option_b = Key {
            alt: true,
            text: Some("∫"),
            ..key("b")
        };
        assert_eq!(send(option_b, KeyEvent::Press, flags(1)), None);
        // GPUI reports Shift+= as "+" without Shift.
        let plus = Key {
            text: Some("+"),
            ..key("+")
        };
        assert_eq!(
            send(plus, KeyEvent::Press, flags(31)).as_deref(),
            Some("\x1b[61:43;2;43u")
        );
        assert_eq!(
            send(plus, KeyEvent::Repeat, flags(31)).as_deref(),
            Some("\x1b[61:43;2:2;43u")
        );
        assert_eq!(send(plus, KeyEvent::Press, flags(1)), None);
        let ctrl_c = Key {
            ctrl: true,
            ..key("c")
        };
        assert_eq!(
            send(ctrl_c, KeyEvent::Repeat, flags(3)).as_deref(),
            Some("\x1b[99;5:2u")
        );
        let ctrl_alt_f = Key {
            ctrl: true,
            alt: true,
            ..key("f")
        };
        assert_eq!(
            send(ctrl_alt_f, KeyEvent::Press, flags(31)).as_deref(),
            Some("\x1b[102;7u")
        );

        assert_eq!(text("你好", flags(31)), b"\x1b[0;;20320:22909u");
        assert_eq!(text("你好", flags(8)), b"");
        assert_eq!(text("你好", flags(1)), "你好".as_bytes());
        assert_eq!(text("\x1b[1;2R", flags(31)), b"\x1b[1;2R");

        let shift = Key {
            shift: true,
            ..Key::default()
        };
        assert_eq!(
            modifier("shift", true, &shift, flags(31)).as_deref(),
            Some(&b"\x1b[57441;2u"[..])
        );
        assert_eq!(
            modifier("shift", false, &Key::default(), flags(31)).as_deref(),
            Some(&b"\x1b[57441;1:3u"[..])
        );
        assert_eq!(modifier("shift", true, &shift, flags(1)), None);
    }
}

//! Keyboard layout utilities for macOS
//!
//! Uses TIS (Text Input Source) APIs to dynamically determine keycodes
//! for the current keyboard layout, enabling layout-independent shortcuts.

use core_foundation::base::{CFRelease, TCFType};
use core_foundation::data::CFData;
use std::collections::HashMap;

// FFI declarations for Carbon/CoreServices APIs
#[link(name = "Carbon", kind = "framework")]
extern "C" {
    fn TISCopyCurrentKeyboardLayoutInputSource() -> *mut std::ffi::c_void;
    fn TISGetInputSourceProperty(
        input_source: *const std::ffi::c_void,
        property_key: *const std::ffi::c_void,
    ) -> *const std::ffi::c_void;
    fn LMGetKbdType() -> u32;
    static kTISPropertyUnicodeKeyLayoutData: *const std::ffi::c_void;
}

#[link(name = "CoreServices", kind = "framework")]
extern "C" {
    fn UCKeyTranslate(
        key_layout_ptr: *const u8,
        virtual_key_code: u16,
        key_action: u16,
        modifier_key_state: u32,
        keyboard_type: u32,
        key_translate_options: u32,
        dead_key_state: *mut u32,
        max_string_length: usize,
        actual_string_length: *mut usize,
        unicode_string: *mut u16,
    ) -> i32;
}

const KUC_KEY_ACTION_DISPLAY: u16 = 3;
const KUC_KEY_TRANSLATE_NO_DEAD_KEYS_BIT: u32 = 0;

/// Default QWERTY keycode for 'V' key
const QWERTY_V_KEYCODE: u16 = 9;

/// Build a lookup table mapping lowercase characters to their keycodes
/// for the current keyboard layout.
fn build_char_to_keycode_map() -> HashMap<char, u16> {
    let mut map = HashMap::new();

    unsafe {
        let input_source = TISCopyCurrentKeyboardLayoutInputSource();
        if input_source.is_null() {
            return map;
        }

        let layout_data_ref =
            TISGetInputSourceProperty(input_source, kTISPropertyUnicodeKeyLayoutData);

        if layout_data_ref.is_null() {
            CFRelease(input_source);
            return map;
        }

        // Wrap the CFData without retaining (it's owned by input_source)
        let layout_data: CFData = TCFType::wrap_under_get_rule(layout_data_ref as *const _);
        let layout_ptr = layout_data.bytes().as_ptr();
        let kbd_type = LMGetKbdType();

        // Iterate through keycodes 0-127 to build reverse lookup
        for keycode in 0u16..128 {
            let mut dead_key_state: u32 = 0;
            let mut char_buf: [u16; 4] = [0; 4];
            let mut actual_len: usize = 0;

            let result = UCKeyTranslate(
                layout_ptr,
                keycode,
                KUC_KEY_ACTION_DISPLAY,
                0, // no modifiers
                kbd_type,
                KUC_KEY_TRANSLATE_NO_DEAD_KEYS_BIT,
                &mut dead_key_state,
                char_buf.len(),
                &mut actual_len,
                char_buf.as_mut_ptr(),
            );

            if result == 0 && actual_len == 1 {
                if let Some(ch) = char::from_u32(u32::from(char_buf[0])) {
                    // Store lowercase version, prefer lower keycodes
                    map.entry(ch.to_ascii_lowercase()).or_insert(keycode);
                }
            }
        }

        CFRelease(input_source);
    }

    map
}

/// Get the keycode for a character in the current keyboard layout.
/// Returns None if the character cannot be found.
pub fn keycode_for_char(ch: char) -> Option<u16> {
    let map = build_char_to_keycode_map();
    map.get(&ch.to_ascii_lowercase()).copied()
}

/// Get the keycode for 'v' in the current keyboard layout.
/// Falls back to QWERTY keycode (9) if lookup fails.
pub fn get_paste_keycode() -> u16 {
    keycode_for_char('v').unwrap_or(QWERTY_V_KEYCODE)
}

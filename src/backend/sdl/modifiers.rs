use crate::KeyModifiers;

pub fn convert_modifiers(modifiers: sdl2::keyboard::Mod) -> KeyModifiers {
	let mut result = KeyModifiers::empty();

	use sdl2::keyboard::Mod as I;

	if modifiers.intersects(I::LALTMOD | I::RALTMOD) {
		result.insert(KeyModifiers::ALT)
	}

	if modifiers.intersects(I::MODEMOD) {
		result.insert(KeyModifiers::ALT_GRAPH)
	}

	if modifiers.intersects(I::CAPSMOD) {
		result.insert(KeyModifiers::CAPS_LOCK)
	}

	if modifiers.intersects(I::LCTRLMOD | I::RCTRLMOD) {
		result.insert(KeyModifiers::CONTROL)
	}

	if modifiers.intersects(I::RGUIMOD | I::LGUIMOD) {
		result.insert(KeyModifiers::META)
	}

	if modifiers.intersects(I::NUMMOD) {
		result.insert(KeyModifiers::NUM_LOCK)
	}

	if modifiers.intersects(I::LSHIFTMOD | I::RSHIFTMOD) {
		result.insert(KeyModifiers::SHIFT)
	}

	result
}

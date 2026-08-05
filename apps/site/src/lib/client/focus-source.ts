const KEYBOARD_NAVIGATION = new Set([
	'Tab',
	'ArrowUp',
	'ArrowDown',
	'ArrowLeft',
	'ArrowRight',
	'Home',
	'End',
	'PageUp',
	'PageDown',
	'Enter',
	' ',
]);

export function installFocusSourceTracker(): () => void {
	const root = document.documentElement;

	function markKeyboard(event: KeyboardEvent) {
		if (KEYBOARD_NAVIGATION.has(event.key)) root.dataset.focusSource = 'kbd';
	}

	function markPointer() {
		root.dataset.focusSource = 'pointer';
	}

	document.addEventListener('keydown', markKeyboard, true);
	document.addEventListener('pointerdown', markPointer, true);

	return () => {
		document.removeEventListener('keydown', markKeyboard, true);
		document.removeEventListener('pointerdown', markPointer, true);
	};
}

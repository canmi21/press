export const DEFAULT_PIXELS_PER_REM = 16;

export function remFromDefaultPixels(value: number): string {
	return `${value / DEFAULT_PIXELS_PER_REM}rem`;
}

export function remFromMeasuredPixels(value: number, measuredRoot?: number): string {
	const root =
		measuredRoot ??
		(typeof document === 'undefined'
			? DEFAULT_PIXELS_PER_REM
			: Number.parseFloat(getComputedStyle(document.documentElement).fontSize) ||
				DEFAULT_PIXELS_PER_REM);
	return `${value / root}rem`;
}

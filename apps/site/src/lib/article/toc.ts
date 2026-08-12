type Schedule = (callback: () => void) => void;
type ScheduleFrame = (callback: () => void) => number;

export const articleHashScript = `(function(){var n=performance.getEntriesByType("navigation")[0],h=location.hash;if(h&&n&&n.type==="navigate"){window.canmiArticleInitialHash=h.slice(1);history.replaceState(history.state,"",location.pathname+location.search)}})()`;

export function scheduleInitialHashJump(
	postRender: Schedule,
	nextFrame: ScheduleFrame,
	cancelFrame: (frame: number) => void,
	reset: () => void,
	jump: () => void,
): () => void {
	let cancelled = false;
	let frame: number | undefined;

	postRender(() => {
		if (cancelled) return;
		reset();
		frame = nextFrame(() => {
			if (!cancelled) jump();
		});
	});

	return () => {
		cancelled = true;
		if (frame !== undefined) cancelFrame(frame);
	};
}

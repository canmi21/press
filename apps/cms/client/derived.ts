export type DerivedReport = {
	classes: Array<{
		id: string;
		name: string;
		detail: string;
		have: number;
		want: number;
		action: string | null;
		paid: boolean;
	}>;
};

type Class = DerivedReport['classes'][number];

function requiredElement<T extends Element>(root: Element, selector: string): T {
	const element = root.querySelector<T>(selector);
	if (element === null) throw new Error(`required element is missing: ${selector}`);
	return element;
}

function toneOf(entry: Class): 'complete' | 'short' | 'empty' {
	if (entry.want === 0 || entry.have >= entry.want) return 'complete';
	return entry.have === 0 ? 'empty' : 'short';
}

function renderClass(entry: Class): HTMLLIElement {
	const item = document.createElement('li');
	const short = entry.want - entry.have;
	item.dataset.tone = toneOf(entry);

	const name = document.createElement('strong');
	name.textContent = entry.name;

	const count = document.createElement('span');
	count.className = 'derived-count';
	count.textContent = `${entry.have}/${entry.want}`;

	const track = document.createElement('span');
	track.className = 'meter-track';
	const fill = document.createElement('span');
	fill.className = 'meter-fill';
	const ratio = entry.want === 0 ? 1 : entry.have / entry.want;
	fill.style.setProperty('--meter-fill', `${(ratio * 100).toFixed(2)}%`);
	track.appendChild(fill);

	const detail = document.createElement('p');
	detail.textContent = entry.detail;

	const footer = document.createElement('div');
	footer.className = 'derived-footer';
	const state = document.createElement('span');
	state.className = 'derived-state';
	state.textContent = short === 0 ? 'Complete' : `${short} outstanding`;
	footer.appendChild(state);

	// The command is shown rather than run. Running it is the task centre's job -- these
	// operations take minutes and several of them spend money, so the thing that starts one has
	// to be able to report progress, refuse a second copy of itself, and record what it cost.
	// A button that could do none of that would be a worse version of copying this line.
	if (entry.action !== null && short > 0) {
		const command = document.createElement('code');
		command.className = 'health-action';
		command.textContent = `cms ${entry.action}`;
		footer.appendChild(command);
	}
	if (entry.paid && short > 0) {
		const paid = document.createElement('span');
		paid.className = 'derived-paid';
		paid.textContent = 'asks a model';
		footer.appendChild(paid);
	}

	const heading = document.createElement('div');
	heading.className = 'derived-heading';
	heading.appendChild(name);
	heading.appendChild(count);

	item.appendChild(heading);
	item.appendChild(track);
	item.appendChild(detail);
	item.appendChild(footer);
	return item;
}

export function renderDerived(root: HTMLElement, report: DerivedReport): void {
	const outstanding = report.classes.reduce(
		(total, entry) => total + Math.max(0, entry.want - entry.have),
		0,
	);
	const behind = report.classes.filter((entry) => toneOf(entry) !== 'complete').length;
	requiredElement<HTMLElement>(root, '[data-derived-total]').textContent =
		`${report.classes.length} record classes`;
	requiredElement<HTMLElement>(root, '[data-derived-state]').textContent =
		outstanding === 0
			? 'Everything derived'
			: `${outstanding} outstanding across ${behind} ${behind === 1 ? 'class' : 'classes'}`;

	const list = requiredElement<HTMLUListElement>(root, '[data-derived-list]');
	list.replaceChildren();
	for (const entry of report.classes) list.appendChild(renderClass(entry));
}

export function renderDerivedError(root: HTMLElement, error: unknown): void {
	requiredElement<HTMLElement>(root, '[data-derived-total]').textContent = '—';
	const state = requiredElement<HTMLElement>(root, '[data-derived-state]');
	state.textContent = error instanceof Error ? error.message : String(error);
	state.dataset.state = 'error';
}

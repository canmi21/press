import { requiredElement } from './dom';

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

export type TaskRun = {
	task: string;
	pid: number;
	shell: 'cli' | 'desktop';
	started: string;
	done: number;
	total: number;
	message: string;
};

type Class = DerivedReport['classes'][number];
type StartTask = (task: 'favicon') => void;

function toneOf(entry: Class): 'complete' | 'short' | 'empty' {
	if (entry.want === 0 || entry.have >= entry.want) return 'complete';
	return entry.have === 0 ? 'empty' : 'short';
}

function renderClass(entry: Class, runs: readonly TaskRun[], startTask: StartTask): HTMLLIElement {
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

	// Only migrated free tasks become controls. Paid tasks stay as text until a warning flow exists,
	// and known-but-unmigrated tasks stay as text until they can use the substrate. See spec/tasks.md.
	if (entry.action !== null && short > 0) {
		const command =
			entry.action === 'favicon' && !entry.paid
				? document.createElement('button')
				: document.createElement('code');
		command.className = 'health-action';
		command.textContent = `cms ${entry.action}`;
		if (command instanceof HTMLButtonElement) {
			command.type = 'button';
			command.dataset.taskAction = 'favicon';
			command.disabled = runs.some((run) => run.task === 'favicon');
			command.addEventListener('click', () => startTask('favicon'));
		}
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

export function renderDerived(
	root: HTMLElement,
	report: DerivedReport,
	runs: readonly TaskRun[],
	startTask: StartTask,
): void {
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
	for (const entry of report.classes) list.appendChild(renderClass(entry, runs, startTask));
}

export function renderTaskRuns(root: HTMLElement, runs: readonly TaskRun[]): void {
	for (const button of root.querySelectorAll<HTMLButtonElement>('[data-task-action]')) {
		button.disabled = runs.some((run) => run.task === button.dataset.taskAction);
	}

	const section = requiredElement<HTMLElement>(root, '[data-task-runs]');
	const list = requiredElement<HTMLUListElement>(root, '[data-task-run-list]');
	section.hidden = runs.length === 0;
	list.replaceChildren();
	for (const run of runs) {
		const item = document.createElement('li');

		const heading = document.createElement('div');
		heading.className = 'task-run-heading';
		const name = document.createElement('strong');
		name.textContent = `cms ${run.task}`;
		const source = document.createElement('span');
		source.textContent = `${run.shell} · pid ${run.pid}`;
		heading.appendChild(name);
		heading.appendChild(source);

		const track = document.createElement('span');
		track.className = 'meter-track';
		const fill = document.createElement('span');
		fill.className = 'meter-fill';
		const ratio = run.total === 0 ? 0 : Math.min(1, run.done / run.total);
		fill.style.setProperty('--meter-fill', `${(ratio * 100).toFixed(2)}%`);
		track.appendChild(fill);

		const detail = document.createElement('div');
		detail.className = 'task-run-detail';
		const message = document.createElement('span');
		message.textContent = run.message || 'Starting…';
		const count = document.createElement('span');
		count.textContent = `${run.done}/${run.total}`;
		detail.appendChild(message);
		detail.appendChild(count);

		item.appendChild(heading);
		item.appendChild(track);
		item.appendChild(detail);
		list.appendChild(item);
	}
}

export function renderDerivedError(root: HTMLElement, error: unknown): void {
	requiredElement<HTMLElement>(root, '[data-derived-total]').textContent = '—';
	const state = requiredElement<HTMLElement>(root, '[data-derived-state]');
	state.textContent = error instanceof Error ? error.message : String(error);
	state.dataset.state = 'error';
}

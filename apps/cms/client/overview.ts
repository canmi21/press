export type OverviewSnapshot = {
	articles: {
		total: number;
		sections: Array<{
			name: string;
			articles: number;
		}>;
	};
	media: {
		referenced: number;
		published: number;
		described: number;
	};
	health: {
		warnings: number;
		notices: number;
		gaps: Array<{
			level: 'warn' | 'info';
			subject: string;
			detail: string;
			action?: 'image' | 'alt' | 'favicon';
		}>;
	};
};

function requiredElement<T extends Element>(root: Element, selector: string): T {
	const element = root.querySelector<T>(selector);
	if (element === null) throw new Error(`required element is missing: ${selector}`);
	return element;
}

function percentage(value: number, total: number): number {
	return total === 0 ? 0 : Math.round((value / total) * 100);
}

function sectionLabel(name: string): string {
	return name
		.split('-')
		.map((part) => `${part.charAt(0).toUpperCase()}${part.slice(1)}`)
		.join(' ');
}

/**
 * How wide a section's bar is drawn, as a fraction of the track.
 *
 * The full width means "all of the writing", not "as much as the biggest section has". Scaling
 * against the largest section is what the previous chart did, and it makes the bar a ranking
 * rather than a quantity: the leader is pinned to the full track whether it holds forty articles
 * or one, so four sections holding one article each drew four full-width bars. That is the
 * ordinary state of a small workspace, not an unlucky one, and a panel that looks identical when
 * full and when nearly empty is one nobody reads twice.
 *
 * Against the total, the same workspace draws four quarter-width bars -- visibly a four-way split
 * -- and the bars keep meaning the same thing as the workspace grows. The cost is that a lopsided
 * workspace makes every minor section very short, which is a true statement about it; the ratio
 * returned here stays exact, and `.section-list .meter-fill` holds the drawn bar to a minimum
 * visible stub so a small share never renders as an absent one.
 *
 * @param articles - Articles in this section.
 * @param total - Articles across every section.
 * @returns A fraction in [0, 1].
 */
function sectionFill(articles: number, total: number): number {
	return total === 0 ? 0 : articles / total;
}

type Meter = {
	label: string;
	value: string;
	fill: number;
	tone?: 'neutral' | 'ready' | 'short';
};

function renderMeters(list: HTMLUListElement, meters: readonly Meter[]): void {
	list.replaceChildren();
	for (const meter of meters) {
		const item = document.createElement('li');
		item.dataset.tone = meter.tone ?? 'neutral';

		const label = document.createElement('span');
		label.className = 'meter-label';
		label.textContent = meter.label;

		const track = document.createElement('span');
		track.className = 'meter-track';
		const fill = document.createElement('span');
		fill.className = 'meter-fill';
		fill.style.setProperty('--meter-fill', `${(meter.fill * 100).toFixed(2)}%`);
		track.appendChild(fill);

		const value = document.createElement('span');
		value.className = 'meter-value';
		value.textContent = meter.value;

		item.appendChild(label);
		item.appendChild(track);
		item.appendChild(value);
		list.appendChild(item);
	}
}

function renderSections(list: HTMLUListElement, snapshot: OverviewSnapshot): void {
	const total = snapshot.articles.total;
	renderMeters(
		list,
		snapshot.articles.sections.map((section) => ({
			label: sectionLabel(section.name),
			value: String(section.articles),
			fill: sectionFill(section.articles, total),
		})),
	);
}

function renderReadiness(list: HTMLUListElement, snapshot: OverviewSnapshot): void {
	const referenced = snapshot.media.referenced;
	const rows = [
		{ label: 'Published', value: snapshot.media.published },
		{ label: 'Described', value: snapshot.media.described },
	];
	renderMeters(
		list,
		rows.map((row) => ({
			label: row.label,
			value: `${row.value}/${referenced}`,
			fill: referenced === 0 ? 0 : row.value / referenced,
			tone: row.value === referenced && referenced > 0 ? ('ready' as const) : ('short' as const),
		})),
	);
}

function renderHealth(root: HTMLElement, snapshot: OverviewSnapshot): void {
	const healthState = requiredElement<HTMLElement>(root, '[data-health-state]');
	const healthList = requiredElement<HTMLUListElement>(root, '[data-health-list]');
	healthList.replaceChildren();

	if (snapshot.health.gaps.length === 0) {
		healthState.textContent = 'Every referenced resource is ready.';
		healthState.dataset.state = 'clear';
		healthState.hidden = false;
		healthList.hidden = true;
		return;
	}

	healthState.hidden = true;
	healthList.hidden = false;
	for (const gap of snapshot.health.gaps) {
		const item = document.createElement('li');
		item.dataset.level = gap.level;
		const marker = document.createElement('span');
		marker.className = 'health-marker';
		marker.setAttribute('aria-label', gap.level === 'warn' ? 'Warning' : 'Notice');
		const copy = document.createElement('div');
		const subject = document.createElement('strong');
		subject.textContent = gap.subject;
		const detail = document.createElement('span');
		detail.textContent = gap.detail;
		copy.appendChild(subject);
		copy.appendChild(detail);
		// The snapshot already names the subcommand that closes each gap; dropping it left the one
		// actionable field on the page unread.
		if (gap.action !== undefined) {
			const remedy = document.createElement('code');
			remedy.className = 'health-action';
			remedy.textContent = `cms ${gap.action}`;
			copy.appendChild(remedy);
		}
		item.appendChild(marker);
		item.appendChild(copy);
		healthList.appendChild(item);
	}
}

export function renderOverview(root: HTMLElement, snapshot: OverviewSnapshot): void {
	const issueCount = snapshot.health.warnings + snapshot.health.notices;
	requiredElement<HTMLElement>(root, '[data-kpi-articles]').textContent = String(
		snapshot.articles.total,
	);
	requiredElement<HTMLElement>(root, '[data-kpi-sections]').textContent =
		`${snapshot.articles.sections.length} sections`;
	requiredElement<HTMLElement>(root, '[data-kpi-media]').textContent = String(
		snapshot.media.referenced,
	);
	requiredElement<HTMLElement>(root, '[data-kpi-published]').textContent =
		`${percentage(snapshot.media.published, snapshot.media.referenced)}%`;
	requiredElement<HTMLElement>(root, '[data-kpi-described]').textContent =
		`${percentage(snapshot.media.described, snapshot.media.referenced)}%`;
	requiredElement<HTMLElement>(root, '[data-kpi-issues]').textContent = String(issueCount);
	requiredElement<HTMLElement>(root, '[data-kpi-issue-detail]').textContent =
		`${snapshot.health.warnings} warnings · ${snapshot.health.notices} notices`;
	requiredElement<HTMLElement>(root, '[data-content-total]').textContent =
		`${snapshot.articles.total} total`;
	requiredElement<HTMLElement>(root, '[data-readiness-total]').textContent =
		`${snapshot.media.referenced} resources`;
	requiredElement<HTMLElement>(root, '[data-health-total]').textContent =
		issueCount === 0 ? 'Clear' : `${issueCount} open`;

	renderSections(requiredElement(root, '[data-content-list]'), snapshot);
	renderReadiness(requiredElement(root, '[data-readiness-list]'), snapshot);
	renderHealth(root, snapshot);
}

export function renderOverviewError(root: HTMLElement, error: unknown): void {
	for (const element of root.querySelectorAll<HTMLElement>('[data-kpi-value]')) {
		element.textContent = '—';
	}
	const healthState = requiredElement<HTMLElement>(root, '[data-health-state]');
	healthState.textContent = error instanceof Error ? error.message : String(error);
	healthState.dataset.state = 'error';
}

import { afterEach, beforeEach, expect, it, vi } from 'vitest';

const initialize = vi.fn();
const render = vi.fn(async (id: string, source: string) => ({
	svg: `<svg id="${id}"><text>${source}</text></svg>`,
}));

vi.mock('mermaid', () => ({ default: { initialize, render } }));

const palette: Record<string, string> = {
	'--mermaid-page': '#fbfbfb',
	'--mermaid-paper': '#ffffff',
	'--mermaid-paper-hover': '#f2f2f2',
	'--mermaid-border': '#e3e3e3',
	'--mermaid-border-strong': '#d3d3d3',
	'--mermaid-text': '#161616',
	'--mermaid-text-soft': '#696969',
	'--mermaid-text-strong': '#0d0d0d',
	'--mermaid-ink': '#1f1f1f',
	'--mermaid-accent': '#2b7fff',
};

beforeEach(() => {
	vi.stubGlobal('getComputedStyle', () => ({
		fontFamily: 'Inter, sans-serif',
		getPropertyValue: (name: string) => palette[name] ?? '',
	}));
});

afterEach(() => {
	vi.unstubAllGlobals();
});

it('initializes strict rendering from the colocated hex palette', async () => {
	const { renderMermaid } = await import('./mermaid');
	const root = { closest: () => null } as unknown as HTMLElement;
	const result = await renderMermaid('flowchart LR\nA --> B', root);

	expect(initialize).toHaveBeenCalledOnce();
	expect(initialize).toHaveBeenCalledWith(
		expect.objectContaining({
			startOnLoad: false,
			securityLevel: 'strict',
			htmlLabels: false,
			theme: 'base',
			themeVariables: expect.objectContaining({
				background: '#f2f2f2',
				primaryColor: '#ffffff',
				lineColor: '#d3d3d3',
				textColor: '#161616',
			}),
		}),
	);
	expect(render).toHaveBeenCalledWith('mermaid-diagram-1', 'flowchart LR\nA --> B');
	expect(result.svg).toContain('mermaid-diagram-1');
});

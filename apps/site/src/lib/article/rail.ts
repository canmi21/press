export function railEndOffset(
	viewportHeight: number,
	railHeight: number,
	articleBottom: number,
): number {
	const restingBottom = (viewportHeight + railHeight) / 2;
	return Math.min(0, articleBottom - restingBottom);
}

export function railTop(viewportHeight: number, railHeight: number, articleBottom: number): number {
	return (
		(viewportHeight - railHeight) / 2 + railEndOffset(viewportHeight, railHeight, articleBottom)
	);
}

export function homeCenter(
	restingCenter: number,
	viewportHeight: number,
	tocHeight: number,
	articleBottom: number,
): number {
	const restingTocTop = (viewportHeight - tocHeight) / 2;
	const offset = railEndOffset(viewportHeight, tocHeight, articleBottom);
	const renderedTocTop = restingTocTop + offset;
	const boundary = offset < 0 ? renderedTocTop : Math.max(0, renderedTocTop);
	return Math.min(restingCenter, boundary / 2);
}

export const articleRailScript = `(function(){requestAnimationFrame(function(){var n=document.querySelector(".toc-nav"),h=document.querySelector(".home-slot"),a=document.querySelector("article"),t=a&&a.querySelector("h1");if(!n||!h||!a||!t||getComputedStyle(n).display==="none")return;var r=parseFloat(getComputedStyle(document.documentElement).fontSize)||16,z=n.getBoundingClientRect().height,b=a.getBoundingClientRect().bottom,o=Math.min(0,b-(innerHeight+z)/2);n.style.setProperty("--toc-end-offset",o/r+"rem");var q=t.getBoundingClientRect(),l=parseFloat(getComputedStyle(t).lineHeight)||q.height,c=q.top+scrollY+l/2,u=(innerHeight-z)/2+o,d=o<0?u:Math.max(0,u),e=Math.min(c,d/2),p=parseFloat(getComputedStyle(h).top);h.style.setProperty("--home-offset",(e-p)/r+"rem")})})()`;

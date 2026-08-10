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

export function homeRestingCenter(
	restingCenter: number,
	viewportHeight: number,
	tocHeight: number,
): number {
	const restingTocTop = (viewportHeight - tocHeight) / 2;
	return Math.min(restingCenter, Math.max(0, restingTocTop) / 2);
}

export function homeCenter(
	restingCenter: number,
	viewportHeight: number,
	tocHeight: number,
	articleBottom: number,
): number {
	return (
		homeRestingCenter(restingCenter, viewportHeight, tocHeight) +
		railEndOffset(viewportHeight, tocHeight, articleBottom)
	);
}

export const articleRailScript = `(function(){requestAnimationFrame(function(){var n=document.querySelector(".toc-nav"),h=document.querySelector(".home-slot"),a=document.querySelector("article"),t=a&&a.querySelector("h1");if(!n||!h||!a||!t||getComputedStyle(n).display==="none")return;var r=parseFloat(getComputedStyle(document.documentElement).fontSize)||16,z=n.getBoundingClientRect().height,b=a.getBoundingClientRect().bottom,o=Math.min(0,b-(innerHeight+z)/2);n.style.setProperty("--toc-end-offset",o/r+"rem");var q=t.getBoundingClientRect(),l=parseFloat(getComputedStyle(t).lineHeight)||q.height,c=q.top+scrollY+l/2,u=(innerHeight-z)/2,e=Math.min(c,Math.max(0,u)/2)+o,p=parseFloat(getComputedStyle(h).top);h.style.setProperty("--home-offset",(e-p)/r+"rem")})})()`;

const DOT = '•';

/**
 * Mail providers whose domain names nobody. Anything absent is treated as a domain the reader
 * controls, where the name is itself the identity. See spec/engagement.md.
 */
const PUBLIC_MAIL_DOMAINS = new Set([
	'163.com',
	'126.com',
	'aliyun.com',
	'aol.com',
	'daum.net',
	'fastmail.com',
	'foxmail.com',
	'gmail.com',
	'gmx.com',
	'gmx.de',
	'googlemail.com',
	'hanmail.net',
	'hey.com',
	'hotmail.com',
	'icloud.com',
	'live.com',
	'mac.com',
	'mail.com',
	'mail.ru',
	'me.com',
	'msn.com',
	'naver.com',
	'outlook.com',
	'pm.me',
	'proton.me',
	'protonmail.com',
	'qq.com',
	'sina.com',
	'sina.cn',
	'tuta.com',
	'tutanota.com',
	'web.de',
	'yahoo.com',
	'yahoo.co.jp',
	'yandex.com',
	'yandex.ru',
	'yeah.net',
	'ymail.com',
	'zoho.com',
]);

/**
 * A recognisable but unreadable form of the reader's own address, for a screen somebody else may
 * be looking at. Enough survives to tell a typo from the intended address.
 */
export function maskEmail(email: string): string {
	const address = email.trim().toLowerCase();
	const at = address.lastIndexOf('@');
	const local = address.slice(0, at);
	const domain = address.slice(at + 1);
	if (at < 1 || !domain) return DOT.repeat(4);
	return `${maskLocal(local)}@${maskDomain(domain)}`;
}

function maskLocal(local: string): string {
	// One character is the whole local part, so revealing the first reveals all of it.
	if (local.length < 2) return DOT;
	return local.slice(0, 1) + DOT.repeat(local.length - 1);
}

function maskDomain(domain: string): string {
	if (PUBLIC_MAIL_DOMAINS.has(domain)) return domain;

	// Only the final label survives. Recovering a registrable domain from an arbitrary one needs
	// the public suffix list, which is a payload this cannot justify -- and guessing at it would
	// leave `example.co.uk` exposed while `example.com` was hidden.
	const labels = domain.split('.');
	const tld = labels.pop();
	if (tld === undefined || labels.length === 0) return DOT.repeat(domain.length);
	return [...labels.map((label) => DOT.repeat(label.length)), tld].join('.');
}

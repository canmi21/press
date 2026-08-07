-- Seeds the counts the four existing articles carried in their frontmatter before the counter
-- existed. Those numbers came from the old site and are not reconstructible from anything
-- here, so they are written once, as data, rather than left to restart from zero.
--
-- `OR IGNORE` rather than a plain insert: reads may already have landed by the time this is
-- applied, and a seed must never overwrite a real count.
INSERT OR IGNORE INTO `article_reads` (`slug`, `count`) VALUES
	('architecture/compile-time-rendering', 5250),
	('development/rust-cargo-cranelift-tuning', 9510),
	('milestone/less-is-more', 2540),
	('mirror/less-than-an-hour', 1950);

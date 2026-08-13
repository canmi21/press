# X lookups

Grok can search X. No other runner can. This file is the decision about what that means for
the CMS, not a description of the four operations; those live in
[x/mod.rs](../apps/cms/src/x/mod.rs).

## A single-provider job is not a runner choice

`Runner` answers which assistant does a piece of work that every assistant can do. Every
variant can translate, summarise, tag. The X tools exist only for Grok.

Putting them on that enum -- a `model_for_x` that returns `None` for five of six variants --
would make the type look like it is choosing among assistants when it is really choosing
whether the job exists. The shape would lie.

So the operations live in their own module and name Grok as a fact. They reuse the grok
binding already in [runner.rs](../apps/cms/src/i18n/runner.rs) because that file owns how
the binary is invoked, not because a runner is being selected. Extra flags that only this
job needs -- no web search, no subagents, a turn cap -- stay with the operation.

Nothing else in the repository has this property yet. The next capability that exists for
only one provider should follow this, not grow a `model_for_*` that is `None` almost
everywhere.

## The model absorbs the tool's shape

The tool's own text is undocumented and unversioned. A parser tied to it breaks silently
when the wording shifts. The model is asked to call the tool and report the result in a
format we specify, so that churn costs a prompt change rather than a parser change.

The format is the line-anchored convention already used for translations; see
[i18n.md](i18n.md). Post text is worse than prose -- emoji, URLs, code, unbalanced
brackets -- which is why JSON is even less appropriate here than it was there. `⟦` and
`⟧` are U+27E6 and U+27E7; [segment.rs](../apps/cms/src/i18n/segment.rs) explains the
choice. The sentinels stay defined there; this module imports them.

A line that *ends* with a marker is still a marker. The workspace voice rule made one
reply open with a Chinese sentence and the count marker on the same line; requiring
the marker to be the whole line would have thrown away a complete transcription.

## Integrity is checked locally

A record count is asked for before the records and checked against how many record blocks
came back. A dropped record fails the reply; a garbled record does not take its siblings
with it.

A post id is a 19-digit snowflake, so a mangled one is visible without another call. User
ids are older and shorter, so they are checked as digits rather than as a fixed width.
Numeric fields parse as numbers or that record is rejected. None of these costs a request.

## Semantic search is silent when the threshold is wrong

`x_semantic_search` is threshold-sensitive. A reasonable query returned nothing at the
tool's own default and produced results at `0.1`. The operation defaults to `0.1` for
that reason: an empty answer more often means the threshold was too high than that nothing
exists.

## No store until something reads one

The operations return a value. They do not write a `data/build` record, do not join the
task catalogue, and have no GUI adapter. Inventing a storage format before there is a
consumer is how you get the wrong one.

## Reached as `cms x`

The four operations sit behind one command named for the vendor, which is the correct
place for a vendor name -- see [naming.md](naming.md). They print JSON on stdout the way
`cms overview` does. Nothing consumes that JSON yet.

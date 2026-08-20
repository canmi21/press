# Twitter lookups

Grok can search Twitter. No other runner can. This file is the decision about what that means for
the CMS, not a description of the four operations; those live in
[twitter/mod.rs](../apps/cms/src/twitter/mod.rs).

## A single-provider job is not a runner choice

`Runner` answers which assistant does a piece of work that every assistant can do. Every
variant can translate, summarise, tag. The Twitter tools exist only for Grok.

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

A line that _ends_ with a marker is still a marker. The workspace voice rule made one
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

## A card is the first reader, so fetched tweets have a store

An article references a tweet as `::twitter{tweet="<id>"}`. The snowflake is globally unique;
including the username in that reference would add a second identity that becomes stale when an
account is renamed. The fetched author belongs to the record instead.

The site resolves that id against `data/build/twitter.json`, a committed snapshot made from
`cms twitter thread` output. It never asks Twitter during a build, in the Worker, or in the
browser. That keeps CI self-contained, avoids one request per reader, and leaves an article
readable if the live tweet later disappears. The snapshot keeps the root tweet only: replies are
separate authored objects, not part of the card the directive requested. Engagement counts are
facts at lookup time and stay that way until somebody deliberately refreshes the snapshot.

A missing record renders the ordinary directive placeholder instead of failing the article.
The article therefore remains the list of wanted tweets, with no second inventory to maintain,
while an unfinished lookup is explicit rather than silently omitted.

## Reached as `cms twitter`

The four operations sit behind one command named for the service, and they print JSON on stdout
the way `cms overview` does. A thread lookup's root tweet may be copied into the card snapshot;
the lookup itself remains a read and never changes the workspace as a side effect.

## It is Twitter here, and the addresses are `twitter.com`

The service renamed itself; this repository did not follow, and that is a decision rather than an
oversight.

**The name.** `X` is a single letter that already means a dozen things in a codebase -- an axis,
an unknown, a placeholder, a coordinate, a cross. `spec/x.md`, `crate::x`, `x_command`, `cms x`:
each of those reads as a variable somebody forgot to name. `Twitter` says which service it is at
every point of use, which is the whole job of a name. A rename that makes identifiers ambiguous
buys currency at the cost of the thing names are for.

**The addresses.** `twitter.com` is still live and permanently redirects, and it will stay that
way: too much of the written web points at it for the owner to drop it. That makes the choice a
free one, paid for with a redirect nobody waits on, and spent on the name that reads.

Two exceptions, both the same rule. **Grok's tool names keep their `x_` spelling** --
`x_user_search` and its siblings are the vendor's identifiers, not this repository's, and
[naming.md](naming.md) puts a vendor's name at the binding edge and nowhere else. And **article
prose is the author's**: what `contents/` calls the service, and which of its hosts an article
links to, is writing rather than configuration, and nothing here reaches into it.

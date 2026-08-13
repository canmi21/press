# Long-running tasks

The desktop CMS is a resident process because it holds the schedule and the editor; see
[architecture.md](architecture.md). This is how the work it schedules is described, kept from
colliding with itself, and observed.

## The catalogue is data, and it is complete before the runner

Every operation that takes more than an instant is declared in
[task/mod.rs](../apps/cms/src/task/mod.rs): what it is, whether it asks a model, what it reads,
what it writes, and which tasks must have run first. Nothing there runs anything.

Splitting the description from the execution is what lets the catalogue be finished first. A GUI
listing what exists, a scheduler ordering it, and a person asking how much a full run would cost
are all answerable now, against thirteen entries, rather than after thirteen operations have been
rewritten. Adding an entry makes a task **known**, not runnable.

Reads and writes name **records**, not paths. Two tasks contend when they mutate the same records,
and stating it that way leaves the file layout free to move without the catalogue following it.

`after` is declared and nothing reads it. The dependency is a fact about the task, so it belongs
beside the task rather than inside whatever eventually orders them. It is tested even though it is
unused: an edge naming a task that does not exist would otherwise stay invisible until a scheduler
deadlocked or silently skipped work, long after anyone remembers writing it. Declaring edges early
is only safe because the test holds them.

Instant reads -- `overview`, `articles`, `derived`, `check`, `port` -- are deliberately absent.
Listing them would put five entries in every task view that can never be watched, waited on, or
scheduled.

## Computing and writing are separate concerns

**A task holds no lock while it thinks.** The expensive part of `cms alt` and `cms tag` is minutes
of model calls; the part that touches `data/media.yaml` is milliseconds at the end. Leasing that
file for the length of a run would serialise the two tasks over a critical section a ten-thousandth
of its length, and serialise them at exactly the point where parallelism is worth the most.

So mutations are values, not writes. A worker finishes an item, pushes the mutation it produced,
and moves on. Applying it is somebody else's job.

**Within a process, one writer per record store applies them in turn.** That removes write races
without any worker waiting on another, and it means the store is only ever open in one place.

**Between processes, a short lock is taken at the moment of applying** -- and only then. The CMS is
several processes by design: the desktop client, a hand-run command in a terminal, and eventually
the schedule. A single-threaded writer inside one of them says nothing about the others, so this
lock is what actually makes the write safe. Held for one apply, it does not measurably contend.

A mutation is flushed as it is applied rather than batched to the end of a run. What is being
protected is paid output: losing an hour of translations to a crash costs money, and the write is
cheap next to the call that produced the value.

## Contention is resolved per item, and the loser does not wait

Work is claimed one item at a time -- a content id, an article, a (segment, locale) pair -- and the
claim is atomic across processes.

**A claimed item is skipped, not waited for.** If A is translating segment one and B wants segment
two, both proceed. If B wants the segment A already holds, B leaves it alone and takes the next.
Only when every item B wanted is already claimed does B report that A is doing this work and exit.

Waiting is the thing to avoid. A queue is an ordering, ordering is the scheduler's job, and the
scheduler does not exist yet -- a task that blocks on a lease would be a scheduler nobody designed,
with no timeout, cancellation or priority. Skipping needs none of those and leaves the run's own
report accurate: it says what it did and what somebody else was already doing.

A task declared `Items::Whole` cannot divide, so a second runner can only stand aside.

### A claim stops concurrent duplication; only re-reading stops sequential duplication

Claims are taken and released per item, so two runs overlapping in time still each do an item the
other finished before it was reached. Measured, with two `cms favicon --force` processes over the
same five domains: each collected three and stood aside on two, which is six pieces of work for
five domains. One domain was fetched twice because the second run arrived after the first had let
go of it.

For favicons that is a wasted request. For anything that asks a model it is the same answer paid
for twice.

So the claim is only half of it. **After taking the claim, an item is re-read from the record
before the work starts**, and an item that has since been done is dropped. The claim makes that
check safe -- nothing can complete between reading and working, because the claim is held across
both -- and the check is what makes the claim mean "still needed" rather than merely "not being
done this instant".

A forced run skips the check by definition: `--force` says redo it, and there is nothing to
observe that would change the answer.

## Rewriting article text is a compatibility path, not the design

`cms image` is the only task that edits `contents/**/*.md`, and a test asserts that. It does so
because an author wrote a temporary filename -- `![](shot.png)` -- and the reference has to become
the content id once the picture is derived. Every other task reading `Articles` declares
`after: ["image"]` largely to stay clear of that rewrite.

**It exists only because there is no editor yet.** An editor that derives a picture at the moment
it is inserted -- store it, then write the content id into the article -- produces an article that
never held a temporary name, so nothing is left to rewrite. The rewrite is compensation for
authoring markdown by hand, not a permanent step.

Two things follow. The split between deriving a picture and writing it is still wanted, because
that is exactly the pair an editor calls synchronously for one image. And `cms image` keeps a
narrower job afterwards: content that arrived without passing through the editor -- a migration, a
batch somebody dropped in, an article written in another editor. That is a run a person starts,
not one a schedule fires, which is what keeps it away from an open draft.

Do not build anything new on the assumption that article text is rewritten behind the author's
back. The `after: ["image"]` edges are expected to weaken once insertion handles its own images.

**Published bytes and their manifest must exist before an article is rewritten.** A crash on the
safe side of that boundary leaves an unreferenced derived image, and another run can finish the
rewrite. The reverse order can leave an article pointing at bytes that do not exist after the
rewrite has destroyed the original filename -- and with it the information a later run needed to
repair the article. The same rule holds when an editor stores one image before inserting its id.

## The editor's round trip is byte-identical, and stays that way by test

**This requirement is being retired.** [i18n.md](i18n.md) now takes a segment id from a block's
canonical form rather than from its bytes, and stores articles already normalised -- so the file
becomes a projection of the canonical form rather than the authority, and the editor is free to
write whatever the normaliser produces. Byte equality was the right bar while the bytes were what
ids were taken from. Until the migration lands, it still is, so what follows continues to hold and
the harness continues to run.

Opening an article and saving it without editing leaves the file unchanged, byte for byte. All
four articles pass. That was not a foregone conclusion and it is not free: it holds because three
specific things were fixed, and it will stop holding the moment a fourth is missed.

It matters here more than in most repositories. A segment id is the hash of the block's text, so a
serializer that rewrites syntax it was not asked to touch silently orphans that block's
translations -- there are over sixteen hundred of them. Whitespace is normalised before hashing, so
reflowing a paragraph is safe; changing a bullet marker or an emphasis character is not. A round
trip that is merely close would do that damage on every open rather than once.

What had to be fixed, and the shape they share:

- Frontmatter needs both a remark plugin to parse it and a schema node to hold it. Parsing alone
  leaves an mdast node the editor refuses, because ProseMirror has nowhere to put it. Nothing reads
  the YAML: not reordering or requoting it is the whole point, since `cms i18n` hashes frontmatter
  values into ids too.
- A code fence's `meta` -- everything after the language word -- is carried by mdast and dropped by
  the stock schema, which keeps only `language`. It is written back as a separate mdast field, not
  appended to the language: a space terminates the language word, so packing both together makes
  remark escape it and the fence returns as `tokei&#x20;title=...`.

**Every one of those is the same journey: an mdast field, a place in the schema to hold it, and a
serializer that writes it back.** That is what a custom block type costs here, and the estimate is
measured rather than guessed. The three directive forms confirm it independently: container,
leaf, and text directives each need their mdast fields carried into a schema node and written back
by a serializer. A parameterised fence is the cheaper variation because it can extend the existing
code-block node instead of adding another node, but its structured parameters still need a schema
attribute and the raw mdast fields still need a serializer.

The stock commonmark preset remains the foundation, not the home for repository syntax. The
project extensions form a preset layered after it, so the directive parser, all three schemas, and
the code-block override are installed as one unit by both the editor and the round-trip harness.
Forking commonmark would make upstream behavior ours to maintain; scattering the extensions would
let the harness and editor silently test different pipelines. Milkdown does not provide a generic
unknown-node escape hatch here: every directive form needs an explicit schema and transformer,
which is the measured cost rather than an implementation accident.

For an opening fence whose info string is `{abc lang}`, remark's lexical split is kept verbatim:
`lang` is `{abc` and `meta` is `lang}`. The schema additionally carries
`{ name: "abc", values: ["lang"] }` for the block owner. Replacing the raw fields with the
interpreted values would require reconstructing the author's spelling at serialization time and
weaken the byte-identical guarantee for no benefit; renderers read the structured attribute
instead. A `font` directive's `family` attribute and a `font` fence's sole value must be ids
exported by `@canmi/fonts`, and are rejected otherwise. That single catalogue prevents the editor
syntax and renderer capabilities from drifting apart. See
[markdown.ts](../apps/cms/client/markdown.ts).

[markdown-roundtrip.ts](../apps/cms/scripts/markdown-roundtrip.ts) checks the claim and reports
what changed by category. It has been confirmed to fail: a `*` bullet marker comes back as `-` and
is caught. Run it after touching the parse or serialize path, because the damage it guards against
is invisible in a diff of the editor's own code.

## A finished run leaves nothing behind, and that is the gap

The registry answers what is running. Nothing answers what ran. An entry disappears when its
process lets go, so a run that failed and a run that succeeded look identical from outside: both
are simply absent.

This is visible today. `favicon::collect` deliberately survives a dead domain and reports it in
`Outcome.failed`, which the CLI prints; the desktop adapter starts the same operation on a thread
and drops the outcome entirely. The counts on the Derived page still tell the truth -- work that
did not happen is still outstanding -- but the reason is gone.

Recording it means run history: what ran, when, how it ended, what it cost. That is the Activity
page, and it needs to be durable, because a history held in memory is emptied by the restart that
most often follows a crash worth reading about. Half of it is worse than none: a control that
reports success it did not verify is the failure mode this whole layer exists to avoid.

So the outcome is dropped on purpose until there is somewhere durable to put it, rather than being
kept somewhere that would have to be unbuilt.

## Liveness is a held lock, never a written status

A run that records `status: running` in a file and is then killed with `SIGKILL`, panics, or loses
power leaves that word behind for good. Nothing can distinguish it from a live run, every later run
refuses to start, and the workspace is poisoned until somebody deletes a file by hand and guesses
whether it was safe to.

So a file records only intent and metadata -- which task, which process, when it started, how far
it got. **The fact of being alive is that the runner still holds an exclusive lock on its own
entry.** Anything wanting to know tries to take that lock: succeeding means the writer is gone and
the entry is a corpse to be reaped. The kernel drops the lock when the process dies however it
dies, which is why this needs no heartbeat, no timeout and no daemon.

Written status can only ever be a hint about a process that was alive when it was written.

## Runtime state lives in `.cms/`, keyed by nothing

The run registry, the claims and the lock files sit in `.cms/` at the repository root, untracked.

Not `data/`: that directory groups assets with the records about them, and the question asked of
anything new there is whether reading its diff would tell anyone anything. A process id and a
progress count fail that test, and putting them there would blur what the directory means even
though the allowlist would keep them out of git.

Not outside the repository either. State held elsewhere has to be keyed by which repository it
belongs to, and a path is not a directory name, so the key becomes a hash -- and then nothing can
tell which checkout a directory belongs to without resolving it. Putting the state _in_ the
repository dissolves the question: processes working on one checkout share a directory because it
is the same directory, and two checkouts are independent because they are.

**A claim file's name is still a hash, and that is a different case.** What made hashing the wrong
answer above was that somebody looking at the directory could no longer tell which repository it
served. An item key -- an article path with slashes in it, a segment id and a locale -- also
cannot be a file name, but nothing needs to recover it _from_ the name: the key is written inside
the file, so `cms claims` and a person reading the directory both get the readable answer. The
hash costs no legibility there, which is the only thing it cost above.

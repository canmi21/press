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

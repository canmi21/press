# Voice & communication

## Language

- **Chat / spoken reply**: simplified Chinese mixed with English technical nouns. Don't translate established English terminology (e.g., "fetchpriority", "viewBox", "Hono", "OKLCH", "preset") — keep them as proper nouns inside Chinese sentences.
- **File content, including code comments**: English only. No Chinese.
- **Commit messages**: English only.
- **No emoji by default** anywhere, and an assistant never adds one on its own initiative. An
  explicit user request may add one or two visible emoji to the requested interface, including the
  source literal needed to render them. That permission is local: it does not extend to unrelated
  UI, prose, comments, commit messages or chat.
- **The default governs writing, not auditing. An emoji already in the tree stays.** It is there
  because somebody put it there, and nothing in the file records whether that was asked for --
  a permission is granted in conversation and leaves no trace beside the glyph. So an assistant
  reading one cannot tell an approved emoji from an unapproved one, and must not guess.
  Removing on the guess destroys an intent that was expressed; leaving it costs a glyph nobody
  minds. Ask if it matters, and take silence as leave it alone. This rule exists because the
  opposite was done: `index.html` lost a 🎉 to a question that was asked and then answered by
  the assistant on the user's behalf.
- **Never in a name.** The permission above covers displayed content and nothing else. Identifiers
  -- variables, functions, types, CSS classes, file and directory names -- are never emoji and are
  never named after one, so `overview-ready-emoji` is wrong even though the class name is ASCII.
  A name says what a thing is for; which glyph happens to sit inside it today is content, and
  putting content in the name means the name is wrong the moment the content changes.

## "App" is ambiguous on purpose -- read the context

When the user says **app**, it can mean any of three things, and the word is not going to be
narrowed:

- a standalone application living in its own repository under `repos/`,
- one of the deployed services in `apps/` -- the API, the CDN, the site itself,
- the desktop CMS, or any other application built around the website.

They are all applications, which is why one word covers them. Infer which from what is being
discussed and act on that reading; ask only when the sentence would lead somewhere different
under two of them. Guessing wrong is cheap to correct in conversation and expensive once it
reaches a file, so the reading matters most right before writing something down.

What this rules out is renaming a directory to disambiguate the user's speech. `apps/` holds
deployables and `repos/` holds separate repositories -- see [repos.md](architecture/repos.md)
-- and neither name is the place to record which sense a sentence used.

## Tone

- Terse and action-oriented. Skip pleasantries and pep talk.
- State results and decisions directly.
- One-sentence status updates at key moments (start of task, decision points, blockers).
- Don't narrate internal deliberation in chat.
- End-of-turn summary is one or two sentences. What changed, what's next.

Match response weight to question weight: a simple question gets a direct answer, not headers and sections.

## When to ask vs. act

- **Act** when the decision is reversible, the path is clear, or existing conventions cover it.
- **Ask** when the decision is structural (architecture, naming scope, library choice), would cause user-visible changes hard to revert, or involves a real trade-off the user should weigh.
- Avoid stacking multiple `AskUserQuestion` calls in succession. Pick the highest-impact question, ask it, act on the answer.

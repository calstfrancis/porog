# Porog — Plan

**Created:** 2026-11-10 · **Status:** Phases 1–6 built and released (0.1.0–0.4.0). Phases 7–14 written
2026-11-17 and **not started**: wait for Cal to say "implement the plan".

Porog (порог, "threshold") is the way back into the Fond suite: Zerkalo, Pereplyot,
Kartoteka, Sputnik and Skrizhal. Same conventions as the other plan files: **read a phase
before starting it, tick its boxes as it lands, commit each phase on its own.** The plan
file is the memory, not the conversation.

---

## 0. What it is for

Porog is not an app grid; the desktop already has one. It exists for one moment: coming
back after a break, a switch or a bad day, and needing to know **where was I, and what is
the one next thing?** Re-entry is where an autistic or executive-function-limited user
pays most: rebuilding a lost focus is expensive, and choosing between five apps and a
dozen recent files is a decision before any work has started.

So Porog shows one Now item, one way back into each app, and one search box, then gets
out of the way.

## 1. Principles

Each change is checked against these. They are the same lenses as Zerkalo's
`AUTISM-UX-PLAN.md` and Sputnik's `CALM-SCHEDULE-PLAN.md`, applied to a launcher.

- **P1 · The layout never moves.** Five rows, one per app, always in the same order. A row
  is never sorted by recency, never hidden, never added. Only its text changes.
- **P2 · Porog never computes an answer another app owns.** The Now item is Sputnik's Now
  card, in Sputnik's words and Sputnik's five states. If the two could disagree, Porog is
  wrong by construction, so it asks rather than re-derives.
- **P3 · A label says exactly what the button does.** "Resume" reopens that file at that
  place. "Open" opens the app. If Resume cannot be honoured, the button says Open.
- **P4 · Nothing changes while you are looking at it.** Porog refreshes when its window
  gains focus, and at no other time.
- **P5 · No demands.** No badges, counts in red, streaks, statistics, notifications or
  "you haven't opened X in a while". A number is plain text, and exact.
- **P6 · One thing by default, more on request.** One item per app, with a `details`
  status-bar toggle for the last three. Same toggle name and behaviour as Sputnik's.
- **P7 · Porog links none of the apps.** It reads small files they write and starts them
  as programs. A Porog release needs no app release, and the reverse.

## 2. The window

```
┌─────────────────────────────────────────────────────────┐
│ ≡                    [ Search the suite   Ctrl+K ]      │
├─────────────────────────────────────────────────────────┤
│ NOW  (from Sputnik)                                     │
│ Read Soskice ch. 3 · THM 501 · about 40 min    [Start]  │
│ 2 more today · 1 needs a decision                       │
├─────────────────────────────────────────────────────────┤
│ Sputnik    Week 10 of Fall 2026               [Open]    │
│ Pereplyot  Soskice — Friendship · p. 47       [Resume]  │
│ Kartoteka  412 references                     [Open]    │
│ Zerkalo    essay-2.typ · yesterday 16:40      [Resume]  │
│ Skrizhal   CV elements                        [Open]    │
├─────────────────────────────────────────────────────────┤
│ details                                         v0.1.0  │
└─────────────────────────────────────────────────────────┘
```

- The Now area keeps its height when Sputnik has nothing for today; it says so in one line
  ("Nothing placed on today.") rather than disappearing (P1).
- Each app row always has its app name as a way to open the app plainly, even when the
  button says Resume: clicking the name opens the app without resuming.
- The rows run in the order the work does: see what is due (Sputnik), read (Pereplyot),
  cite (Kartoteka), write (Zerkalo), with the CV (Skrizhal) last. Sputnik's row sits
  directly under the Now area, which is Sputnik's too.
- Keyboard: Porog opens with the first row focused. Up/Down move between rows, Enter
  and Alt+1…5 (row order) do what the row's button says, Resume or Open; clicking the app's
  name always opens it plainly. A single click
  acts only on a button; a row needs a double click.
- Porog stays open after starting an app. It is a normal app, not an autostart.

## 3. The data contract

Settled 2026-11-10: status files, plus one Sputnik helper command. Porog links no app
crate and reads no app's private files (`session.json`, `gui.json`, `recents.json`).

### 3.1 Status files

Each app writes `~/.local/share/fond/status/<app>.json` whenever its resume target changes
(a file opened, a document closed, quit). Written atomically: temp file, then rename.

```json
{
  "schema": 1,
  "app": "zerkalo",
  "written": "2026-11-10T16:40:12-04:00",
  "summary": "412 references",
  "resume": [
    {
      "title": "essay-2.typ",
      "detail": "Essay 2 project",
      "at": "2026-11-09T16:40:00-04:00",
      "args": ["/home/cal/Documents/Zerkalo/essay-2/essay-2.typ"]
    }
  ]
}
```

- `resume` is newest first, at most three entries. Porog shows the first, or all three
  with `details` on. An empty or missing `resume` means the row shows `summary` (or the
  app's fixed purpose) and the button says Open.
- `args` is what Porog hands back to the app, unchanged, as `flatpak run <id> <args…>`.
  The app decides how it is resumed; Porog knows no app's command line.
- Porog checks that a path in `args[0]` still exists before offering Resume (P3).
- The schema lives in a new `fond-status` crate in `fond-core`, with the same compatibility
  rules as the annotation sidecar: new fields additive and optional, unknown fields
  ignored, `schema` bumped only for a change an old reader cannot safely ignore.
- The directory is under `~/.local/share`, which every Fond flatpak already reaches through
  `--filesystem=home`.

### 3.2 Sputnik's Now item

A status file would be wrong after midnight, so Porog asks Sputnik each time its window
gains focus, through a helper in Sputnik's flatpak (the `pereplyot-extract-text` pattern):

```
flatpak run --command=sputnik-now io.github.calstfrancis.Sputnik
```

It prints one JSON object and exits: the Now item (title, course, minutes, the `args` that
start it), the count of other items on today, and the count needing a decision. The helper
calls the same `sputnik-core` code as Sputnik's own Now card. It must not touch the keyring
or the network, and must return in well under a second.

Three literal outcomes when there is no item to show: "Nothing placed on today.",
"Sputnik is not installed.", and "This Sputnik is too old to show today here."

## 4. Phases

### Phase 1 — the fixed board
- [x] Workspace: `crates/porog-core` (no GTK) and `porog-ui-gtk`
- [x] Five fixed rows that open each app's flatpak; "Not installed" when it is absent
- [x] Install check and launch run off the GTK main thread; refresh on window focus only
- [x] Vendored `style/fond.css`, registered in `fond-style/sync.sh` and `check.sh`
- [x] House chrome: hamburger popover, System/Light/Dark toggle persisted to
      `~/.config/porog/config.json`, version button opening the live `CHANGELOG.md`, About
- [x] Keyboard: Up/Down between rows, Enter activates, Alt+1…5 for the five apps
- [x] Flatpak manifest (with `--talk-name=org.freedesktop.Flatpak`), desktop file,
      metainfo, icon, `cargo-sources.json`
- [x] `check-versions.sh`, `dev-build.sh`, `publish-flatpak.sh`, `publish-flatpak-local.sh`,
      `ci.yml`, CI-published `release-flatpak.yml`, `RELEASE-CI-SETUP.md`
- [x] Cal: create the `calstfrancis/porog` GitHub repo and add it as `origin`, then the one-time
      secrets setup in `RELEASE-CI-SETUP.md`
- [ ] Test launching from inside the flatpak: the `flatpak-spawn --host` path has only been
      exercised by unit tests on its arguments. Moved to Phase 7.
- [x] Welcome / What's New window, `capture-screenshots.sh`, `release-preflight.sh`

### Phase 2 — Resume
- [x] `fond-status` crate in `fond-core`: types, atomic write, tolerant read, tests
- [x] Porog reads status files off the main thread and shows Resume / Open per P3
- [x] Pereplyot writes its status (reading position as `detail`)
- [x] Zerkalo writes its status (the document in front, on open and tab switch)
- [x] Sputnik, Kartoteka, Skrizhal write a `summary` only: "Week 10 · Fall 2026" (with
      `summary_until` = midnight, so Porog never shows last week's number), "412 references",
      "3 CV entries". All three use `fond_status::summary`, a shared background writer.
- [x] `details` status-bar toggle
- [x] Committed in all six repos; every app depends on `fond-status` as a git `rev` of fond-core,
      with `packaging/cargo-sources.json` regenerated.
- [x] Docs: CHANGELOG/README in all six apps, Porog's own CHANGELOG/README/CLAUDE.md, §2
      keyboard wording, fond-core README.
- [x] Porog's `packaging/cargo-sources.json`, regenerated once `fond-status` was a git dependency.
- [ ] Known limit: Kartoteka's and Skrizhal's counts are as of their last run.

### Phase 3 — Now
- [x] `sputnik-now` helper in Sputnik (`sputnik-gtk --now`, wrapped as a flatpak command),
      sharing the Now card's code: the card's wording moved into `sputnik-core`
      (`TodayAgenda::now_summary`), and the card calls the same functions
- [x] Now area in Porog: Sputnik's card in Sputnik's words, and the literal messages "Sputnik is
      not installed.", "This Sputnik is too old to show today here." (also any unreadable or
      newer reply) and "Sputnik did not answer." (5 s). Sputnik's own "Nothing left for today."
      is shown as is: P2 means Porog does not substitute "Nothing placed on today.".
- [x] Deviation: the button is **Open Sputnik**, not Start. Starting an item (the study timer)
      happens inside a running Sputnik and Sputnik takes no command-line arguments, so a Start
      here would not do what its label said (P3). Revisit with Phase 5, when Sputnik gets
      arguments for starting an item.
- [ ] Known limit: "Not now" on Sputnik's card lasts only while Sputnik is open, so a skipped
      item still shows in Porog.

### Phase 4 — Search (Ctrl+K)
- [x] Decided (Claude, Cal: "do whichever you recommend"): a helper command per app, not reading
      files. Kartoteka first: `kartoteka-gtk --search-json` / flatpak `kartoteka-search`, reading its
      existing index; `kartoteka:KEY` selects a reference.
- [x] The other four (2026-11-12, unreleased): `sputnik-search` (Vault::search, opens `sputnik:` links),
      `pereplyot-search` (library titles + annotations, opens file + `--annotation=`),
      `zerkalo-search` (`.typ` files in work_dir by name/text, opens the file), `skrizhal-search`
      (CV entries, opens `skrizhal:KEY`). Porog asks all five in parallel.
- [x] One box (Ctrl+K), results grouped by app in the fixed app order, Enter opens in that app.
      Asks after a 450 ms pause; the window keeps one size.

### Phase 5 — Cross-app actions
- [x] Decided (Cal, 2026-11-14): the Now card only; "New essay" makes a blank `.typ` named for the
      assignment; "Start reading" just opens the file (Pereplyot already remembers the place).
- [x] Sputnik's `--now` reply gains an optional `action` `{label, app, args}`: a `key:` reading with
      its file or a PDF/EPUB material → Pereplyot with the same arguments Sputnik's own timeline uses;
      a paper assignment → Zerkalo `--new-document="COURSE: Assignment"`. Porog drops an action with no
      label or for an app it does not know, and shows the button beside *Open Sputnik*.
- [x] Zerkalo `--new-document=TITLE`: creates the file in `work_dir` (reads only that setting, never
      the keyring) and starts as if given that file.
- [x] Tested end to end (unreleased, 2026-11-14) with a stand-in `flatpak` running the dev builds:
      the button appears for a paper, a click starts Zerkalo, the file is created and opened. Reading
      actions checked as Sputnik output only (material and library key), not clicked.
- [ ] Not done: the study timer is not started by "Start reading" (Sputnik takes no commands); the
      real flatpak sandbox path; releases of Sputnik, Zerkalo and Porog.

### Phase 6 — Re-entry helpers (Cal, 2026-11-16)
- [x] A note for each app: one line in Porog's own config (`notes`, 160 characters), written from a
      pencil button on the row and shown under it. Porog-only: no app reads it (P7), and nothing
      expires or nags (P5). Adding one makes that row taller; Porog never does it for you (P1, P4).
- [x] Drop a file on the window: PDF/EPUB → Pereplyot, `.typ` → Zerkalo, by extension
      (`porog_core::app_for_file`). Anything else, or several files, gets a toast. Kartoteka and
      Skrizhal take no file argument, so they are not drop targets.
- [x] Sputnik's `--now` reply gains an optional `later` list (up to three `{title, detail}`: due soon,
      then coming up, earliest first, without the Now item); Porog shows it under the card with
      `details` on (P6).
- [x] Tested headless with a stand-in `flatpak` and the dev builds: note saved and shown, `later` lines
      shown, a dragged `.typ` started Zerkalo with its path, a dragged `.bib` got the toast.
- [ ] Not done: the real flatpak sandbox. A file dropped on the sandboxed Porog arrives as a document-portal
      path; whether the other app's sandbox can read that path is untested, and Porog's manifest has no
      home access by design. Moved to Phase 7.

---

## Plan II — Phases 7–14 (written 2026-11-17)

Phases 1–6 gave Porog its shape. Phases 7–14 make it trustworthy, deepen the one thing it is
for (getting back into work), and let it be reached without opening its window. P1–P7 still
hold. Each phase lists the other repos it touches. Apps change first and Porog copes with an
older app (fields stay optional, P7), so no release has to wait for another.

Order: Phase 7 first (D10). After that, the numbered order, though 10 and 11 do not depend on
8 or 9.

Out of scope on purpose (D14): notifications, streaks, statistics, sorting by recency, an
unread dot or "new" badge on anything, starting at login, energy or capacity tracking (Sputnik
and Chered own those), and one button that opens several windows (D8).

### Phase 7 — Trust what is there
Fixes first: a launcher you cannot trust undoes everything else.

- [x] **"New essay" stops making duplicates.** Today Zerkalo's `--new-document=TITLE` always takes a
      free name (`zerkalo/src/new_document.rs`), so a second click, or coming back the next day,
      makes "… Essay 2.typ", and Sputnik keeps offering "New essay" after the draft exists.
      - Zerkalo: when `<stem>.typ` already exists in `work_dir`, open it instead of making a new
        file. Never overwrite. Update its tests and README.
      - Sputnik: the label becomes **Write essay**, which is true whether the draft is new or not
        (P3). Update `now_helper.rs` and its test.
      - Porog: nothing changes; the label is Sputnik's.
- [x] **A click shows it registered.** After a launch button is pressed it reads "Starting…" and
      ignores further presses until Porog's window loses focus (the app's window took it) or 4 s
      pass. If `flatpak run` exits with an error within those 4 s, a toast says "Zerkalo did not
      start." `porog_core::launch` reports an early exit through a channel instead of only
      reaping the child. The same applies to the Now card's buttons and search results (search
      closes its window, so only the early-exit toast there).
- [ ] **The real sandbox, tested with Cal.** Install the current flatpak and check by hand: Open,
      Resume (the host `test -e` check), the Now card, search, the Now card's action button, and a
      dropped file. Nothing here has run in the real sandbox yet.
- [x] **Dropped files in the sandbox.** A file dropped on the sandboxed Porog arrives as a
      document-portal path the target app most likely cannot see. Ask the portal for the real
      path (`org.freedesktop.portal.Documents.GetHostPaths`) and hand that to the app, whose
      sandbox has home access. If the portal cannot answer, a toast says "Porog cannot pass this
      file on. Open it from the app instead." Outside a sandbox, nothing changes.
- [x] **Helpers never outlive their question.** `helper::run` kills the process when it times
      out, and a search that a newer query replaced kills its remaining helpers instead of
      letting them finish.
- [x] **Every message says what to do next.** "This Sputnik is too old to show today here.
      Update Sputnik to 0.X or later." Same for each app's search message. The versions come from
      one table in `porog-core` (`MIN_VERSIONS`: app × feature), checked against each app's
      CHANGELOG when written and added to Porog's release checklist in `CLAUDE.md`.
- [x] **Groundwork for the later phases:** split `window.rs` (771 lines) into `row.rs` (`AppRow`),
      `now_area.rs` (`NowArea`) and `window.rs` (assembly, actions), with no change in behaviour.
- [x] README: remove the doubled "Clicking an app's name always opens it plainly" paragraph.
- Touches: Zerkalo, Sputnik, Porog.

### Phase 8 — Better cues for getting back in
A file name is a weak cue. A line from the work itself (the heading you were under, the passage
you last marked) is a strong one: interruption research (Altmann & Trafton's memory-for-goals
model) finds that concrete cues from the task shorten the time to resume, and for an
autistic or monotropic user, re-entry is the costly moment (§0).

- [x] `fond-status`: `ResumeEntry` gains an optional `context` (one line, at most 120
      characters, cut by the writer). An additive field, so no schema bump. Tests for the round
      trip and for a file without it.
- [x] Zerkalo writes `context` for the document in front: the nearest heading above the cursor,
      then the start of the sentence the cursor is in, e.g. `§2 Method · "…the second argument
      fails because"`. Written whenever it already writes the status file; no extra writes.
      Quotes from the document are fine (D9).
- [x] Pereplyot writes `context`: the text of the newest highlight in that document, or else the
      chapter or section title at the current position.
- [x] Porog shows `context` as one dim line under the first entry, ellipsized, always (it is part
      of the one item, not another item, so P6 holds), and under each older entry with `details`
      on. The Resume button's tooltip carries it too.
- [x] **Notes carry their date.** A note shows "· left 3 Nov" after it, in the same words as the
      row times (`status::describe_when`). Notes stay per app (D11): the row is the place that
      never moves, while a note tied to one document would vanish from view as soon as another
      document came first, which is when it is needed. Config: `notes` becomes `{text, at}` per
      app, reading the old plain-string form without loss.
- Touches: fond-core (`fond-status`), Zerkalo, Pereplyot, Porog. Each app moves to the new
  fond-core rev on its own; an app still on the old rev writes no `context`, and Porog shows
  nothing in its place.

### Phase 9 — The assignment's pieces
When the Now item is an assignment, the things needed for it live in three apps. Sputnik knows
what they are; Porog shows them (P2).

- [x] Sputnik's `--now` reply gains an optional `pieces` list, at most four, each
      `{label, title, app, args}`: the **Draft** (Zerkalo, `--new-document=COURSE: Title`, which
      after Phase 7 opens the draft if it exists), then the assignment's **readings** (Pereplyot,
      the same arguments Sputnik's own timeline uses). Only pieces whose file or library entry is
      there right now.
- [x] Investigated, not done: a **References** piece opening Kartoteka on the course's
      references. Kartoteka takes only one `kartoteka:KEY` today, so this needs a new Kartoteka
      argument (e.g. `--keys=a,b,c` showing just those). Do it only if that argument is small.
      Result: it is not small (a multi-key argument means a filtered library view in Kartoteka, not
      just argument parsing), so no References piece. Revisit if Kartoteka grows such a view.
- [x] Porog shows the pieces under the Now card with `details` on, one line each, each with its
      own button. No "open all" button (D8). Unknown apps and unlabelled pieces are dropped, as
      with `action`.
- Touches: Sputnik, perhaps Kartoteka, Porog.

### Phase 10 — Each app's What's New, from its row (Cal, 2026-11-17)
- [x] A small icon on every row, between the note pencil and the main button, opens that app's
      What's New in a Porog window: "What's new in Zerkalo 0.59.0 'Release Name'", then the
      headline-and-detail bullets, in the app's words. The icon is the same on every row and
      never changes: no dot, no badge, no "unread" state (P5, D14). Tooltip and accessible name:
      "What's new in Zerkalo". Insensitive when the app is not installed.
- [x] Asked for, like the Now card: each app ships `<app>-whats-new` (flatpak `--command`), which
      prints `{schema: 1, version, release_name?, items: [{headline, detail}]}` and exits, without
      opening a window (D12).
      - Zerkalo, Pereplyot, Sputnik: from the same list their own What's New window uses
        (`welcome_window.rs`, `welcome.rs`, `whats_new.rs`), so the two cannot disagree. Move the
        list into a function both call where it is not already shared.
      - Kartoteka, Skrizhal (no What's New window): the newest `CHANGELOG.md` entry, embedded at
        build time, each `- **Headline.** detail` bullet as one item and any other bullet as a
        headline alone.
- [x] Porog: `porog-core/src/whats_new.rs` (types, parse, tests, using `helper::run`) and a window
      reusing `welcome.rs`'s bullet layout. An app too old to answer: "This Zerkalo is too old to
      show what's new here. Update Zerkalo to 0.X or later." (Phase 7's table.)
- [x] Porog's own row of icons stays as it is: its own What's New stays in the menu.
- Touches: all five apps, Porog.

### Phase 11 — Reach Porog from outside its window (Cal, 2026-11-17)
No GNOME Shell search provider: Cal does not use Activities search (D13).

- [ ] **System tray icon, off by default.** A menu switch "Show in system tray", saved in
      `config.json` (`tray: bool`).
      - Built on StatusNotifierItem (the `ksni` crate, on its own thread, talking to the GTK side
        through a channel). The icon is Porog's own and never changes (P5).
      - Its menu: **Show Porog**, then one line per app in row order ("Resume Zerkalo —
        essay-2.typ" or "Open Kartoteka", the same words and rule as the row's button, P3),
        **Search…** (shows Porog with the search box), **Quit Porog**. Rebuilt when a status file
        changes (a file monitor on the status directory), never reordered.
      - With the tray on, closing the window hides it and Porog keeps running (`app.hold()`);
        the switch's subtitle says so in plain words. With it off, closing quits, as now.
      - GNOME shows no tray icons without the AppIndicator extension. Porog checks whether
        `org.kde.StatusNotifierWatcher` is on the session bus; if not, the switch's subtitle says
        "Your desktop shows no tray icons. On GNOME, install the AppIndicator extension." and the
        switch still works once the extension is there.
      - Manifest: `--talk-name=org.kde.StatusNotifierWatcher` and
        `--own-name=org.kde.StatusNotifierItem-*`.
- [ ] **Command line:** `porog --resume=APP`, `porog --open=APP` and `porog --search[=WORDS]`,
      handed to the running Porog if there is one (GApplication command-line handling), with
      APP the status name (`zerkalo`). The README shows how to bind one to a GNOME keyboard
      shortcut (`flatpak run io.github.calstfrancis.Porog --resume=zerkalo`).
- [ ] Open question, for Cal (O1): with the tray on, should Porog also be offered a "start at
      login" switch? D5 says Porog does not start at login, so not without Cal saying so.
- Touches: Porog only.

### Phase 12 — The suite at a glance, and the first run
- [ ] **"Not installed" leads somewhere.** The row's button becomes **How to install**, opening a
      small dialog: the exact `flatpak install calstfrancis <id>` command with a Copy button, and
      **Open in Software** (`appstream://<id>`).
- [ ] **A Suite window** (menu: "The suite"): one line per app with its installed version and,
      for each thing Porog can ask of it (status file, Now, search, What's New, context lines),
      whether it answers, found by asking, not by version guessing. Each missing one says the
      fix ("Update Zerkalo to 0.X or later"). Run off the main thread, on opening only (P4).
- [ ] **A first run that fits what is installed.** The Welcome window lists the five apps with
      installed or not, and says plainly that rows fill in as each app is used ("Zerkalo will
      show your last document here once you have opened one").
- Touches: Porog only.

### Phase 13 — Keyboard, screen reader, search
- [ ] Shortcuts window (Ctrl+?, also in the menu) listing every key.
- [ ] On a focused row: **N** edits its note, **W** opens its What's New.
- [ ] Screen reader pass with Orca: each row read as one sentence ("Zerkalo. essay-2.typ,
      yesterday 16:40. Resume."), the Now card read as a whole, toasts announced.
- [ ] Checked at 200% text size, at 360 px width, and with animations off.
- [ ] Search: the matched words in bold (escaped markup); each app's group shows "Asking…" until
      it answers, so results appear as each app replies instead of all at once (groups keep the
      fixed order); the last query is kept, selected, when search is reopened.
- [ ] Search: under each group, **Search in Kartoteka for "words"** continues inside the app, for
      each app that takes a `--search=WORDS` argument. Add that argument to an app only where its
      own search box can be filled from outside without much work; list which in this file.
- Touches: Porog, and some apps for `--search=`.

### Phase 14 — Engineering
- [ ] Remove the `unsafe` `set_data` in `search.rs`: keep each row's target in a `Vec` indexed by
      row, or a small `glib::BoxedAnyObject`.
- [ ] **Shared example replies.** `fond-core/contracts/` holds JSON examples of every helper reply
      and the status file (`now/*.json`, `search/*.json`, `whats-new/*.json`, `status/*.json`). Porog
      parses each in a test; each app checks that what it prints has the same fields. Plain files
      rather than shared Rust types, so no app has to move to a new fond-core rev to stay in step.
- [ ] Headless smoke test in CI: Xvfb plus the fake `flatpak` from `capture-screenshots.sh`,
      checking the window builds, Now and rows fill, and a launch reaches the fake.
- [ ] Porog `CLAUDE.md`: `MIN_VERSIONS` and the contract examples in the release checklist.
- Touches: fond-core, all five apps (tests only), Porog.

## 5. Decisions

Settled 2026-11-10:

| | Decision |
|---|---|
| D1 | The Now card is in scope. |
| D2 | Status files plus a Sputnik helper; no linked app crates. |
| D3 | The name is Porog. |
| D4 | The five Fond apps only. No rows for Gost, Iskra or Rubric. |
| D5 | A normal app. It does not start at login. |
| D6 | Row order: Sputnik, Pereplyot, Kartoteka, Zerkalo, Skrizhal — the order the work runs in. Cal had no preference; chosen by Claude. Fixed from here on. |
| D7 | Porog stays open after Start, Resume or Open. |

Settled 2026-11-17 (Plan II):

| | Decision |
|---|---|
| D8 | No button opens several apps at once; each piece of an assignment has its own button (Cal). |
| D9 | Status files may carry short quotes from the user's documents as `context` (Cal). |
| D10 | Phase 7 (trust) comes first (Cal). |
| D11 | Notes stay per app, now with the date they were left. Chosen by Claude at Cal's request: the row never moves, a per-document note would disappear when another document comes first. |
| D12 | An app's What's New is asked for through an `<app>-whats-new` helper and shown in a Porog window, not by starting the app with a flag: it does not open a whole app to read a list, and works the same for apps with no What's New window. Chosen by Claude. |
| D13 | No GNOME Shell search provider: Cal does not use Activities search. |
| D14 | The out-of-scope list at the top of Plan II stays out of scope. |

Open:

- **O1.** With the tray on, offer "start at login"? Not without Cal saying so (D5).
- Search source (was open until Phase 4): settled as a helper command per app.

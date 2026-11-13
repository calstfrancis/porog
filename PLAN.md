# Porog — Plan

**Created:** 2026-11-10 · **Status:** Phases 1–3 built (Phase 1 except the items that need a GitHub remote); nothing committed or released

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
- [ ] Cal: create the `calstfrancis/porog` GitHub repo and add it as `origin` (the manifest
      builds from it, so no flatpak build is possible before this), then the one-time
      secrets setup in `RELEASE-CI-SETUP.md`
- [ ] First dev build, then test launching from inside the flatpak: the
      `flatpak-spawn --host` path has only been exercised by unit tests on its arguments
- [ ] Not started: Welcome / What's New window, `capture-screenshots.sh`,
      `release-preflight.sh`. Add before the first real release.

### Phase 2 — Resume
- [x] `fond-status` crate in `fond-core`: types, atomic write, tolerant read, tests
- [x] Porog reads status files off the main thread and shows Resume / Open per P3
- [x] Pereplyot writes its status (reading position as `detail`)
- [x] Zerkalo writes its status (the document in front, on open and tab switch)
- [x] Sputnik, Kartoteka, Skrizhal write a `summary` only: "Week 10 · Fall 2026" (with
      `summary_until` = midnight, so Porog never shows last week's number), "412 references",
      "3 CV entries". All three use `fond_status::summary`, a shared background writer.
- [x] `details` status-bar toggle
- [ ] Not done: all of it is uncommitted in six repos, and every app depends on
      `fond-status` by path (`../fond-core/crates/fond-status`). Once Cal pushes fond-core,
      switch each to a git `rev` and regenerate each `packaging/cargo-sources.json`; no
      flatpak can build before that.
- [x] Docs: CHANGELOG/README in all six apps, Porog's own CHANGELOG/README/CLAUDE.md, §2
      keyboard wording, fond-core README.
- [ ] Not done: Porog's `packaging/cargo-sources.json` (regenerate once `fond-status` is a git
      dependency; doing it now would vendor a path the flatpak cannot see).
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
- [ ] "Start this reading" (Sputnik item → Pereplyot), "New essay for this assignment"
      (Sputnik item → Zerkalo). Design after phases 2–4 are in daily use.

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

Open (waits for Phase 4):

1. **Search source.** Status files do not carry 412 references. Either each app gets a
   `--search-json <query>` helper, or Porog reads the plain files directly (Kartoteka's
   vault is plain files by design; Sputnik's vault is not meant to be read by others).

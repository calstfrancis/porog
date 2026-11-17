# Changelog

## [0.4.0] "Left Note" — 2026-11-17 — Notes, dropped files, what is due next

### Added
- **A note for each app.** A pencil beside each row's button opens a one-line box: write what you meant to do
  next ("next: rewrite the intro") and it shows in italics under that row until you clear it. It is yours:
  kept in Porog's own settings, never sent to the app.
- **Drop a file on the window.** A PDF or EPUB opens in Pereplyot and a `.typ` file in Zerkalo. Any other
  file, or more than one, gets a short message instead of a guess.
- **What is due next.** With `details` on, the Now card also lists up to three dated things after the Now
  item, in Sputnik's own words. Needs Sputnik 0.27.0.
- The window opens a little taller, so five rows and the extra lines fit.

## [0.3.0] "Ready Hand" — 2026-11-15 — Actions on the Now card

### Added
- **Actions on the Now card.** When Sputnik's Now item is a reading it can open, or a paper assignment,
  the card shows a second button beside *Open Sputnik*: **Start reading** opens the file in Pereplyot
  (where you left off), **New essay** starts a blank document named for the course and assignment in
  Zerkalo. Sputnik says which button, if any, goes with the item; Porog only runs it. Needs Sputnik's
  and, for *New essay*, Zerkalo's next release.

## [0.2.0] "Wide Threshold" — 2026-11-14 — Search reaches every app

### Changed
- **Search asks all five apps.** Sputnik (class notes, topics, materials), Pereplyot (documents and
  your highlights and notes in them), Zerkalo (`.typ` files in the work folder) and Skrizhal (CV
  entries) now answer Ctrl+K as Kartoteka already did, each through its own `<app>-search` command
  (needs the matching app release). The apps are asked at the same time, so a slow one no longer
  holds up the others.

## [0.1.0] "Open Door" — 2026-11-11 — The way back into the Fond suite

### Added
- **Resume.** Each row reads the app's status file (`~/.local/share/fond/status/<app>.json`) and
  shows what you were last doing: Pereplyot's document and page, Zerkalo's document, Sputnik's
  week, the size of the Kartoteka library or CV file. The button says *Resume* only when it can
  reopen that file; otherwise it says *Open*. A dated line (Sputnik's week) disappears at midnight.
- A `details` toggle in the status bar shows the last three entries per app, each with its own
  Resume. Kept between runs.
- **Now.** The top of the window shows Sputnik's Now card, in Sputnik's words: the class about
  to start, the next piece of work, or "Nothing left for today.", with "2 more today · 1 needs a
  decision" beneath. It is asked for each time the window gains focus (a status file would be
  wrong after midnight) and keeps its height, so the rows never move. When there is no card it
  says why in one line: "Sputnik is not installed.", "This Sputnik is too old to show today
  here." or "Sputnik did not answer.".
- **Search.** Ctrl+K (or the magnifier in the header) opens one box that asks the apps and groups
  the answers by app. It asks when typing pauses, never on every key, and the window and the line
  under the box keep their size. Enter opens the first result in its app; Down moves through them.
  Only Kartoteka answers so far, through its `kartoteka-search` command; an app that cannot answer
  is not listed.
- Alt+1 to Alt+5 and Enter do what the row's button says. Clicking the app's name always opens
  the app plainly.

- One window with a fixed row for each Fond app, in the order the work runs: Sputnik,
  Pereplyot, Kartoteka, Zerkalo, Skrizhal. Each row opens its app, or says "Not installed".
- Keyboard: Up and Down move between rows, Enter opens the focused row's app, and Alt+1 to
  Alt+5 open the five apps directly.
- A System / Light / Dark theme choice in the menu, remembered between runs.
- The version number at the bottom right opens this changelog.
- Rows are re-checked when the window gains focus, never while it is being looked at.
- Flatpak packaging and CI workflows. Not built or published yet.
- A Welcome window on the first run and What's New after an update, also under the menu.

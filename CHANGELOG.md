# Changelog

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

# Changelog

## [0.5.0] "Steady Threshold" — 2026-11-17 — The way back, made trustworthy

### Added
- **Keyboard and screen reader.** Ctrl+? lists every shortcut; N on a focused row opens its note, W its What's
  New. Each row and the Now card are announced as one sentence ("Zerkalo. An Example Essay · today 16:40.
  Resume."). The window now fits down to 360 px wide.
- **Search shows each app as it answers.** Every app's group is there at once, saying "Asking…", and fills in as
  its app replies, so a slow one no longer holds the rest back. The words you searched for are in bold, and
  reopening the box brings back your last words, selected.
- **"Not installed" leads somewhere.** The button on an app that is not installed says *How to install* and
  opens the two commands to run, with *Copy commands* and *Open in Software*.
- **The suite.** Menu → *The suite* lists each app with its installed version and whether it answers each thing
  Porog asks of it (status file, Sputnik's Now card, search, What's New), found by asking. A thing an app does
  not answer says what to update.
- **A first run that fits what is installed.** The Welcome window lists the five apps, which are installed,
  and what each row will show once you have used the app.
- **A system tray icon, if you want one.** The menu's *Show in system tray* puts Porog's icon in the tray. Its
  menu is Show Porog, one line per app saying what the row's button says ("Resume Zerkalo — essay-2.typ"),
  Search… and Quit. With the icon on, closing the window only hides it and Porog keeps running; turning it off
  restores quit-on-close. Off by default. GNOME shows tray icons only with the AppIndicator extension; the menu
  says so when none is found.
- **Reach a row from the keyboard or a script.** `porog --resume=zerkalo` does what that row's button says,
  `--open=zerkalo` opens the app plainly, `--search=WORDS` asks the suite and `--search-box` opens the search
  box. Bind one to a shortcut in GNOME Settings (see the README). It does not raise an existing Porog window.
- **What's new in each app, from its row.** A small icon beside each app's note pencil opens that app's What's
  New in a Porog window: the newest release's name and what changed, in the app's own words. The icon is the
  same on every row and never shows a dot or a count. Needs each app's next release; an older one says so,
  and which version to update to.
- **Where you were, in a line.** Under a row's entry, one dim line from the work itself when the app gives
  one: Zerkalo's heading and the sentence you were in, Pereplyot's newest highlight. With `details` on the
  older entries show theirs too. Needs the next Zerkalo and Pereplyot.
- **Notes show when they were left** ("next: rewrite the intro · left 3 Nov"), so you can tell how old one is
  without anything telling you it is old. Notes written before this have no date and show as before.

- **The pieces of an assignment.** With `details` on, a paper on the Now card lists what it is made of, each
  with its own *Open*: the draft in Zerkalo, then the readings the assignment links to in Pereplyot. No button
  opens them all at once. Needs the next Sputnik.

### Changed
- **A button shows it was pressed.** Resume, Open and the Now card's buttons read "Starting…" and ignore
  further presses until the app is running (at most three seconds). If the app exits with an error in that
  time, a message says "… did not start." instead of nothing.
- **A dropped file reaches the app from inside the sandbox.** Porog asks the document portal for the file's
  real path, since the portal's own path is not visible to the other app. If it cannot, it says so.
- **Messages say what to do.** "This Sputnik is too old to show today here. Update Sputnik to 0.24.2 or later."
  The same for each app's search.
- A search that a newer query replaced, or whose window closed, stops its helper processes instead of leaving
  them to finish; a helper that times out is killed.

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

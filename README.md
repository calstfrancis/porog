# Porog

Porog (порог, "threshold") is the way back into the Fond suite: one window with a fixed
row for each of Sputnik, Pereplyot, Kartoteka, Zerkalo and Skrizhal.

It is at an early stage. Each row opens its app, or says "Not installed", and shows what you
were last doing there with a *Resume* button when the app can reopen it, and the top of the window
shows Sputnik's Now card. Ctrl+K searches the whole suite; see `PLAN.md`.

## Using it

| Key | What it does |
|---|---|
| Up / Down | Move between the rows |
| Enter | Do what the focused row's button says (Resume or Open) |
| Ctrl+K | Search the suite; Enter opens the first result |
| Alt+1 … Alt+5 | The same, for Sputnik, Pereplyot, Kartoteka, Zerkalo, Skrizhal |

Clicking an app's name always opens it plainly. The pencil beside a row's button leaves a one-line note
for yourself that shows under that row until you clear it. Drop a PDF or EPUB on the window to read it in
Pereplyot, or a `.typ` file to edit it in Zerkalo. The `details` word in the status bar shows the
last three things per app. A button reads "Starting…" until its app is running.

The menu (top right) has the System / Light / Dark theme choice, *The suite* (each app's version and what it answers), the changelog and About.
The version number at the bottom right also opens the changelog. The theme choice is kept
in `~/.config/porog/config.json`.

## From the tray and the command line

*Show in system tray* in the menu (off by default) keeps an icon. A left click drops down a list: Show
Porog, then each installed Fond app. Click an app's name to open it, or switch to it if it is already
running; when it has something to resume, a line under it ("Resume — essay-2.typ") reopens that. Search… and
Quit are at the bottom. While the icon is on, closing the window hides it and Porog keeps running. GNOME shows tray icons only with the AppIndicator extension.

```sh
porog --resume=zerkalo   # what the Zerkalo row's button says: Resume or Open
porog --open=kartoteka   # open the app plainly
porog --search=grace     # search the suite for "grace"
porog --search-box       # open the search box
```

The app is named as in its status file: `sputnik`, `pereplyot`, `kartoteka`, `zerkalo`, `skrizhal`. To bind
one to a key in GNOME: Settings → Keyboard → Custom Shortcuts → command
`flatpak run io.github.calstfrancis.Porog --resume=zerkalo`. A request for a row does not raise an
existing Porog window; the point is to get to the app.

## Where the rows come from

Each app writes `~/.local/share/fond/status/<app>.json` (the `fond-status` crate in
`fond-core`). Porog reads those files, checks that a file to resume still exists, and hands the
app's own arguments back to `flatpak run`. Porog links none of the apps.

The Now card at the top is asked for, not stored: each time the window gains focus Porog runs
`flatpak run --command=sputnik-now io.github.calstfrancis.Sputnik`, which prints Sputnik's Today
card as one line of JSON (needs a Sputnik that has the command). When the item is a reading or a paper
assignment the reply also carries an `action` (`label`, `app`, `args`); Porog shows it as a button and
hands `args` back to `flatpak run` for that app (*Start reading* → Pereplyot, *New essay* → Zerkalo's
`--new-document=TITLE`).

Search is asked for too: `flatpak run --command=<app>-search <app id> WORDS` (`sputnik-search`,
`pereplyot-search`, `kartoteka-search`, `zerkalo-search`, `skrizhal-search`) prints that app's hits as
one line of JSON. A hit carries either its `args` (Pereplyot: the file and `--annotation=ID`;
Zerkalo: the file; Sputnik: a `sputnik:SEMESTER/COURSE/KIND[/FILE]` link) or a `key` that Porog turns
into `kartoteka:KEY` / `skrizhal:KEY`. Enter hands them back to `flatpak run`.

## Requirements

- GTK 4.10 or later and libadwaita 1.4 or later
- The Fond apps installed as flatpaks from the personal repo. Porog starts them with
  `flatpak run`; it does not link or bundle them.

## Build and run

```sh
cargo build
cargo run -p porog-ui-gtk
cargo test
```

The flatpak manifest is in `packaging/`. It builds from the GitHub repository, which does
not exist yet, so there is no flatpak build so far. Once it does: `./dev-build.sh`.

## Layout

| Path | What it is |
|---|---|
| `crates/porog-core` | The app registry, how each app is launched, and the config file. No GTK. |
| `porog-ui-gtk` | The window. Its `Cargo.toml` carries the app version. |
| `porog-ui-gtk/style/fond.css` | Vendored copy of `fond-style`'s stylesheet. |
| `packaging/` | Flatpak manifest, desktop file, metainfo, icon, vendored cargo sources. |
| `PLAN.md` | Design, data contract and phases. |

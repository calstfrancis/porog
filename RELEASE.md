# Porog v0.5.0 "Steady Threshold"

Install via Flatpak:

```bash
flatpak remote-add --user calstfrancis \
  https://calstfrancis.github.io/flatpak/calstfrancis.flatpakrepo
flatpak install calstfrancis io.github.calstfrancis.Porog
```

Already installed? Update with:

```bash
flatpak update io.github.calstfrancis.Porog
```

---

### What's new

Porog is easier to trust and does more of the remembering.

**Where you were.** Under the document a row offers to resume, one line from the work itself: Zerkalo's heading and sentence, or your newest highlight in Pereplyot. Notes now show when you left them. **What's new in each app** opens from a small icon on every row, never with a dot or a count.

**Reach it your way.** An optional system tray icon (off by default; GNOME needs the AppIndicator extension), `porog --resume=zerkalo` and friends for a keyboard shortcut, N and W on a focused row, and Ctrl+? for every key. The window now fits down to 360 px.

**Fewer surprises.** Buttons read *Starting…* and say if an app did not start. A missing app's button says *Install…* and shows the commands. *The suite* in the menu shows what each app answers. Messages say which version to update to, search shows each app as it answers with your words in bold, and a dropped file reaches its app from inside the sandbox. A paper on the Now card lists its draft and readings with *details* on.

Needs the matching app releases for the new lines: Zerkalo 0.60.0, Pereplyot 0.25.0, Sputnik 0.28.0, Kartoteka 0.25.0, Skrizhal 0.6.0.

### Full changelog

See [CHANGELOG.md](CHANGELOG.md).

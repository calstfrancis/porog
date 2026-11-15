# Porog v0.3.0 "Ready Hand"

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

The Now card can now carry a button beside *Open Sputnik*. When Sputnik's current item is a reading it can open,
*Start reading* opens the file in Pereplyot where you left off. When it is a paper assignment, *New essay* starts a
blank document in Zerkalo named for the course and assignment. Sputnik decides which button, if any, goes with the
item; Porog only runs it.

Needs Sputnik 0.26.0 and, for *New essay*, Zerkalo 0.59.0. With older versions the card looks as before.

---

### Full changelog

See [CHANGELOG.md](CHANGELOG.md).

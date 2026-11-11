# Porog — Claude Instructions

Porog follows the house style and release conventions in `~/Projects/CLAUDE.md`. It is a
Fond suite app. This file only covers what is Porog-specific.

## What this app is

The launcher and dashboard for the Fond suite (Zerkalo, Pereplyot, Kartoteka, Sputnik,
Skrizhal). Read `PLAN.md` before any work: it holds the design principles (P1–P7), the
data contract, the phases and the settled decisions.

## Rules that are easy to break

- **Porog links none of the apps.** No `sputnik-core`, `fond-bib` or other app crate as a
  dependency, and no reading of an app's private files. It reads the status files the apps
  write and starts them with `flatpak run` (`PLAN.md` §3).
- **The five rows never reorder, hide or multiply.** `porog_core::APPS` is the order.
- **Nothing refreshes while the window is focused.** Refresh on focus gain only.
- **Every command and file read runs off the GTK main thread** (`gio::spawn_blocking`),
  including `flatpak info` and launching.
- **Never build or release unless Cal explicitly says to.**

- **The Now card is Sputnik's, not Porog's.** Porog shows what `sputnik-now` prints and adds
  nothing but the three literal messages in `porog-core/src/now.rs`. Never compute it here.

- **Search is asked for, like the Now card.** Porog reads no app's files; each searchable app
  ships a helper command (`porog-core/src/search.rs` `SOURCES`). Show each app's answer in its words.

## Version

`porog-ui-gtk/Cargo.toml`'s `version` is the sole source of truth. `porog-core` keeps its
own version.

## Packaging

CI-published flatpak, same shape as Pereplyot's (`release-flatpak.yml`,
`publish-flatpak-local.sh`, `RELEASE-CI-SETUP.md`). Whenever `Cargo.lock` changes,
regenerate `packaging/cargo-sources.json` per the root `CLAUDE.md`. The manifest grants only
`--filesystem=xdg-data/fond:create` (no home access; the host is asked whether a resume path
exists). `fond-status` is a path dependency (`../fond-core/crates/fond-status`) until fond-core is
pushed; switch it to a git `rev` and regenerate `cargo-sources.json` before any flatpak build.

## Not set up yet

No GitHub remote and no CI secrets, so no flatpak has been built: "prep dev build" can
commit and tag, but `./dev-build.sh` fails until Cal creates `calstfrancis/porog`. No
Welcome / What's New window, `capture-screenshots.sh` or `release-preflight.sh` yet.

## Testing the window headlessly

Launch under Xvfb with `GDK_BACKEND=x11`, `WAYLAND_DISPLAY` unset, `dbus-run-session`,
`ADW_DISABLE_PORTAL=1` (without it the window takes too long to map) and a compositing
window manager (`xfwm4 --compositor=on`), then capture with `import -window <id>`. Put a
fake `flatpak` script first on `PATH` so nothing real is launched, and point
`XDG_CONFIG_HOME` at a throwaway dir.

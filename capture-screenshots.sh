#!/usr/bin/env bash
# capture-screenshots.sh — refresh screenshots/porog-main*.png (light and dark) from a throwaway
# profile, fictional status files and a fake `flatpak`, never your own data. Nothing real is
# launched: the fake `flatpak` only answers. Needs a built target/release/porog
# (cargo build --release), Xvfb, xdotool and ImageMagick. If ../calstfrancis.github.io already
# shows Porog (porog.png at its root) the refreshed PNG/WebP pair is dropped there too.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
BIN="${1:-$HERE/target/release/porog}"
OUT="$HERE/screenshots"
WEBSITE_DIR="${WEBSITE_DIR:-$HERE/../calstfrancis.github.io}"
WORK="$(mktemp -d)"
[ -x "$BIN" ] || { echo "build first: cargo build --release (no binary at $BIN)"; exit 2; }
for tool in Xvfb xdotool import convert python3 dbus-run-session; do
    command -v "$tool" >/dev/null || { echo "missing tool: $tool"; exit 2; }
done
mkdir -p "$OUT"
cleanup() { [ -n "${PGID:-}" ] && kill -TERM -- "-$PGID" 2>/dev/null; [ -n "${XVFB_PID:-}" ] && kill "$XVFB_PID" 2>/dev/null; rm -rf "$WORK"; }
trap cleanup EXIT

cat >"$WORK/session.conf" <<'CONF'
<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN" "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig><type>session</type><keep_umask/><listen>unix:tmpdir=/tmp</listen>
<policy context="default"><allow send_destination="*" eavesdrop="true"/><allow eavesdrop="true"/><allow own="*"/></policy></busconfig>
CONF
Xvfb -displayfd 3 -screen 0 1000x700x24 3>"$WORK/display" >/dev/null 2>&1 &
XVFB_PID=$!
for _ in $(seq 50); do [ -s "$WORK/display" ] && break; sleep 0.1; done
export DISPLAY=":$(tr -d '\n' <"$WORK/display")"

# A fake `flatpak`: every app is installed, and Sputnik's Now card is fictional.
mkdir -p "$WORK/bin" "$WORK/docs" "$WORK/status"
cat >"$WORK/bin/flatpak" <<'FAKE'
#!/bin/sh
if [ "$2" = --command=sputnik-now ]; then
  echo '{"schema":1,"now":{"state":"work","title":"Example Reader, An Example Chapter (pp. 40-61)","detail":"EXA 101 · free for 40 min, until EXA 102 at 14:00","more_today":2,"needs_decision":1}}'
fi
exit 0
FAKE
chmod +x "$WORK/bin/flatpak"

# Fictional status files; the documents they point at have to exist to be resumable.
: >"$WORK/docs/example-reader.pdf"; : >"$WORK/docs/example-essay.typ"
python3 - "$WORK" <<'PY'
import json, sys, datetime
work = sys.argv[1]
now = datetime.datetime.now().astimezone().replace(microsecond=0)
def at(minutes): return (now - datetime.timedelta(minutes=minutes)).isoformat()
midnight = (now + datetime.timedelta(days=1)).replace(hour=0, minute=0, second=0).isoformat()
files = {
    "sputnik": {"summary": "Week 10 · Fall 2026", "summary_until": midnight},
    "pereplyot": {"resume": [{"title": "An Example Reader", "detail": "p. 47 of 312", "at": at(95), "args": [f"{work}/docs/example-reader.pdf"]}]},
    "kartoteka": {"summary": "412 references"},
    "zerkalo": {"resume": [{"title": "An Example Essay", "detail": "example-essay.typ", "at": at(30), "args": [f"{work}/docs/example-essay.typ"]}]},
    "skrizhal": {"summary": "3 CV entries"},
}
for app, body in files.items():
    json.dump({"schema": 1, "app": app, "written": at(5), **body}, open(f"{work}/status/{app}.json", "w"))
PY

start_app() {
    local scheme="$1"
    rm -rf "$WORK/h"; mkdir -p "$WORK/h"/{home,data,config/glib-2.0/settings,cache}
    printf "[org/gnome/desktop/interface]\ncolor-scheme='%s'\n" "$scheme" >"$WORK/h/config/glib-2.0/settings/keyfile"
    env -u WAYLAND_DISPLAY GDK_BACKEND=x11 GTK_A11Y=none ADW_DISABLE_PORTAL=1 GSETTINGS_BACKEND=keyfile \
        POROG_NO_WELCOME=1 FOND_STATUS_DIR="$WORK/status" PATH="$WORK/bin:$PATH" \
        HOME="$WORK/h/home" XDG_DATA_HOME="$WORK/h/data" XDG_CONFIG_HOME="$WORK/h/config" XDG_CACHE_HOME="$WORK/h/cache" \
        setsid dbus-run-session --config-file="$WORK/session.conf" -- "$BIN" >"$WORK/app.log" 2>&1 </dev/null &
    PGID=$!
}
stop_app() { [ -n "${PGID:-}" ] && { kill -TERM -- "-$PGID" 2>/dev/null; sleep 0.4; kill -KILL -- "-$PGID" 2>/dev/null; PGID=""; }; }
window_named() {
    for _ in $(seq 100); do
        for w in $(xdotool search --onlyvisible --name '' 2>/dev/null); do
            case "$(xdotool getwindowname "$w" 2>/dev/null)" in $1) echo "$w"; return 0 ;; esac
        done
        sleep 0.2
    done
    return 1
}
shot() { # shot WINDOW FILE — the window's own rectangle
    local g; g="$(xdotool getwindowgeometry --shell "$1")"
    eval "$g"
    import -window root -crop "${WIDTH}x${HEIGHT}+${X}+${Y}" +repage "$2" 2>/dev/null
}

for scheme in ${SCHEMES:-default prefer-dark}; do
    suffix=""; [ "$scheme" = prefer-dark ] && suffix="-dark"
    start_app "$scheme"
    w="$(window_named 'Porog')" || { echo "no Porog window"; exit 1; }
    sleep 5
    xdotool mousemove 900 650
    sleep 1
    shot "$w" "$OUT/porog-main$suffix.png"
    stop_app
done

echo "wrote $(ls "$OUT"/porog-*.png | wc -l) screenshots to $OUT"
if [ -e "$WEBSITE_DIR/porog.png" ]; then
    cp "$OUT/porog-main.png" "$WEBSITE_DIR/porog.png"
    cp "$OUT/porog-main-dark.png" "$WEBSITE_DIR/porog-dark.png"
    convert "$OUT/porog-main.png" -quality 80 "$WEBSITE_DIR/porog.webp"
    convert "$OUT/porog-main-dark.png" -quality 80 "$WEBSITE_DIR/porog-dark.webp"
    echo "also copied to $WEBSITE_DIR (review and commit there yourself)"
fi

#!/usr/bin/env bash
# smoke-test.sh — start the real window under Xvfb with a fake `flatpak` and fictional status
# files, and check the things that only a running window shows: it builds, asks Sputnik for the
# Now card, resumes a document from the command line, and rejects an unknown option. Nothing
# real is launched. Needs a built target/debug/porog (cargo build), Xvfb, dbus-run-session.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
BIN="${1:-$HERE/target/debug/porog}"
[ -x "$BIN" ] || { echo "build first: cargo build (no binary at $BIN)"; exit 2; }
for tool in Xvfb dbus-run-session; do
    command -v "$tool" >/dev/null || { echo "missing tool: $tool"; exit 2; }
done
W="$(mktemp -d)"
trap 'kill "${XVFB:-0}" 2>/dev/null; rm -rf "$W"' EXIT
mkdir -p "$W/bin" "$W/docs" "$W/status" "$W/config/porog" "$W/home" "$W/data"
echo '{"tray": true}' >"$W/config/porog/config.json"
: >"$W/docs/essay.typ"

cat >"$W/bin/flatpak" <<FAKE
#!/bin/sh
echo "\$@" >> "$W/flatpak.log"
[ "\$1" = info ] && exit 0
case "\$2" in
  --command=sputnik-now) echo '{"schema":1,"now":{"title":"Final essay","detail":"","more_today":0,"needs_decision":0}}' ;;
  --command=*) exit 1 ;;
esac
exit 0
FAKE
chmod +x "$W/bin/flatpak"
cat >"$W/status/zerkalo.json" <<JSON
{"schema":1,"app":"zerkalo","resume":[{"title":"Essay","args":["$W/docs/essay.typ"]}]}
JSON
cat >"$W/session.conf" <<'CONF'
<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN" "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig><type>session</type><keep_umask/><listen>unix:tmpdir=/tmp</listen>
<policy context="default"><allow send_destination="*" eavesdrop="true"/><allow eavesdrop="true"/><allow own="*"/></policy></busconfig>
CONF

Xvfb -displayfd 3 -screen 0 1000x800x24 3>"$W/display" >/dev/null 2>&1 &
XVFB=$!
for _ in $(seq 50); do [ -s "$W/display" ] && break; sleep 0.1; done
export DISPLAY=":$(tr -d '\n' <"$W/display")"
export GDK_BACKEND=x11 GTK_A11Y=none ADW_DISABLE_PORTAL=1 POROG_NO_WELCOME=1
export HOME="$W/home" XDG_CONFIG_HOME="$W/config" XDG_DATA_HOME="$W/data" FOND_STATUS_DIR="$W/status"
export PATH="$W/bin:$PATH"
unset WAYLAND_DISPLAY

timeout 60 dbus-run-session --config-file="$W/session.conf" -- bash -c "
  '$BIN' >'$W/porog.out' 2>&1 &
  sleep 6
  '$BIN' --resume=zerkalo
  sleep 2
  for name in \$(busctl --user list --no-legend | awk '/^:/{print \$1}'); do
    busctl --user call \"\$name\" /MenuBar com.canonical.dbusmenu GetLayout iias 0 5 0 2>/dev/null && break
  done >'$W/tray-menu.out'
  '$BIN' --bogus >'$W/bogus.out' 2>&1; echo \$? >'$W/bogus.exit'
  pkill -TERM -P \$\$ -f '$BIN' 2>/dev/null; true
"

fail=0
check() { if grep -q -- "$2" "$W/flatpak.log"; then echo "ok   $1"; else echo "FAIL $1"; fail=1; fi; }
check "asked Sputnik for the Now card" "--command=sputnik-now"
check "--resume reached the running window and started Zerkalo" "run io.github.calstfrancis.Zerkalo $W/docs/essay.typ"
if [ "$(cat "$W/bogus.exit" 2>/dev/null)" != 0 ]; then echo "ok   unknown option refused"; else echo "FAIL unknown option accepted"; fail=1; fi
if grep -q 'Zerkalo' "$W/tray-menu.out" 2>/dev/null && grep -q 'Resume.*Essay' "$W/tray-menu.out"; then echo "ok   the tray menu lists installed apps, with a resume line"; else echo "FAIL tray menu"; cat "$W/tray-menu.out" 2>/dev/null | head -5; fail=1; fi
if grep -q "panicked" "$W/porog.out"; then echo "FAIL the window panicked"; cat "$W/porog.out"; fail=1; else echo "ok   no panic"; fi
exit $fail

#!/usr/bin/env bash
# Disposable guest only. Real packaged Settings and real update worker; local curl fixture.
set -euo pipefail
: "${SCRAWL_GUI_BINARY:?missing staged AppImage}"
: "${SCRAWL_GUI_OUTPUT:?missing evidence directory}"
while IFS='=' read -r key value; do
  case "$key" in DISPLAY|WAYLAND_DISPLAY|HYPRLAND_INSTANCE_SIGNATURE|DBUS_SESSION_BUS_ADDRESS|XDG_RUNTIME_DIR) export "$key=$value";; esac
done < <(systemctl --user show-environment)
: "${DISPLAY:?missing XWayland display}"
: "${WAYLAND_DISPLAY:?missing compositor display}"
: "${HYPRLAND_INSTANCE_SIGNATURE:?missing user session}"

if systemctl --user list-units --all --type=service | grep -Ei 'apparent.*(running|active)' || pgrep -af '[a]pparent app'; then
  echo 'Another Apparent instance owns this disposable desktop' >&2
  exit 1
fi
mkdir -p "$HOME/update-fixture/bin"
export APPARENT_CONFIG="$HOME/update-fixture/config.json"
printf '{"onboarding":{"seen":true},"updateChannel":"stable"}\n' > "$APPARENT_CONFIG"
chmod 600 "$APPARENT_CONFIG"
export GUI_FIXTURE="$HOME/update-fixture"
export PATH="$GUI_FIXTURE/bin:$PATH"
export APPIMAGE_EXTRACT_AND_RUN=1
chmod 700 "$SCRAWL_GUI_BINARY"
cp "$SCRAWL_GUI_BINARY" "$GUI_FIXTURE/source.AppImage"
sha256sum "$SCRAWL_GUI_BINARY" > "$SCRAWL_GUI_OUTPUT/initial-package.sha256"
[[ "$(sha256sum "$GUI_FIXTURE/source.AppImage" | cut -d' ' -f1)" == "$(cut -d' ' -f1 "$SCRAWL_GUI_OUTPUT/initial-package.sha256")" ]]
cat > "$GUI_FIXTURE/bin/curl" <<'PY'
#!/usr/bin/env python3
"""Return only fixture bytes for exact update endpoints; never contact the network."""
import hashlib
import json
import os
from pathlib import Path
import sys
import time

root = Path(os.environ['GUI_FIXTURE'])
args = sys.argv[1:]
if '--url' not in args:
    sys.exit('fixture: non-update download disabled')
url = args[args.index('--url') + 1]
out = Path(args[args.index('--output') + 1]) if '--output' in args else None
name = 'Apparent-99.0.0-linux-x86_64.AppImage'
image = root / 'source.AppImage'
prefix = 'https://github.com/tryapparent/apparent/releases/download/'
with (root / 'requests.log').open('a') as log:
    print(url, file=log)
if (root / 'hold').exists():
    (root / 'waiting').write_text(url)
    for _ in range(200):
        if not (root / 'hold').exists():
            break
        time.sleep(.1)
    else:
        sys.exit('fixture gate was not released')
mode = (root / 'mode').read_text().strip()
if mode == 'error':
    sys.exit('fixture: update feed unavailable')
if url == 'https://api.github.com/repos/tryapparent/apparent/releases/latest':
    version = '99.0.0' if mode == 'available' else '0.0.0'
    asset = f'Apparent-{version}-linux-x86_64.AppImage'
    assets = [{'name': a, 'browser_download_url': prefix + f'v{version}/' + a}
              for a in (asset, asset + '.sha256')]
    sys.stdout.write(json.dumps({'tag_name': f'v{version}', 'assets': assets}))
elif url == prefix + 'v99.0.0/' + name + '.sha256':
    sys.stdout.write(f'{hashlib.sha256(image.read_bytes()).hexdigest()}  {name}\n')
elif url == prefix + 'v0.0.0/Apparent-0.0.0-linux-x86_64.AppImage.sha256':
    sys.stdout.write('0' * 64 + '  Apparent-0.0.0-linux-x86_64.AppImage\n')
elif url == prefix + 'v99.0.0/' + name and out:
    with image.open('rb') as source, out.open('wb') as dest:
        while data := source.read(1024 * 1024):
            dest.write(data)
else:
    sys.exit(f'unexpected update URL: {url}')
PY
chmod 700 "$GUI_FIXTURE/bin/curl"
printf 'no-update\n' > "$GUI_FIXTURE/mode"
"$SCRAWL_GUI_BINARY" app > "$SCRAWL_GUI_OUTPUT/app.log" 2>&1 &
launcher_pid=$!
trap 'kill "$launcher_pid" 2>/dev/null || true; if [[ -n "${app_pid:-}" ]]; then kill "$app_pid" 2>/dev/null || true; fi; pkill -P "$launcher_pid" 2>/dev/null || true' EXIT
for _ in {1..100}; do
  [[ -S "$XDG_RUNTIME_DIR/apparent.sock" ]] && break
  kill -0 "$launcher_pid" 2>/dev/null || { echo 'packaged app exited before IPC' >&2; exit 1; }
  sleep .2
done
[[ -S "$XDG_RUNTIME_DIR/apparent.sock" ]] || { echo 'packaged app never owned IPC' >&2; exit 1; }
app_pid=$(python3 - "$APPARENT_CONFIG" <<'PY'
import json,sys
print(json.load(open(sys.argv[1]))['running']['pid'])
PY
)
readlink "/proc/$app_pid/exe" > "$SCRAWL_GUI_OUTPUT/running-executable.txt"
appimage_path=$(tr '\0' '\n' < "/proc/$app_pid/environ" | sed -n 's/^APPIMAGE=//p')
[[ "$appimage_path" == "$SCRAWL_GUI_BINARY" ]] || { echo "unexpected active AppImage: $appimage_path" >&2; exit 1; }
[[ "$(sha256sum "$appimage_path" | cut -d' ' -f1)" == "$(cut -d' ' -f1 "$SCRAWL_GUI_OUTPUT/initial-package.sha256")" ]]
grep -q 'appimage_extracted_' "$SCRAWL_GUI_OUTPUT/running-executable.txt"
"$SCRAWL_GUI_BINARY" app # a second invocation asks the owner to open Settings
cat > "$GUI_FIXTURE/controller.py" <<'PY'
import csv
import io
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import struct

out, fixture, config, pid = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3]), int(sys.argv[4])
last_image = None

def click(x, y):
    assert last_image is not None
    with last_image.open('rb') as image:
        assert image.read(16).startswith(b'\x89PNG'), 'not a screenshot'
        width, height = struct.unpack('>II', image.read(8))
    print(f'QMP_CLICK {x} {y} {width} {height}', flush=True)
    assert sys.stdin.readline().strip() == 'CLICK_DONE', 'job-local QMP click was not acknowledged'
    time.sleep(.35)
    pointer = json.loads(subprocess.check_output(['hyprctl', 'cursorpos', '-j']))
    assert abs(pointer['x'] - x) < 10 and abs(pointer['y'] - y) < 10, (pointer, x, y)

def screenshot(name):
    global last_image
    path = out / (name + '.png')
    subprocess.run(['grim', str(path)], check=True, stdout=subprocess.DEVNULL)
    last_image = path
    window = settings_window()[0]
    x, y = window['at']
    width, height = window['size']
    detail = out / (name + '-detail.png')
    subprocess.run(['ffmpeg', '-hide_banner', '-loglevel', 'error', '-y', '-i', str(path),
                    '-vf', f'crop={width}:{height}:{x}:{y},scale={width*2}:{height*2}',
                    '-frames:v', '1', str(detail)], check=True)
    text = subprocess.check_output(['tesseract', str(detail), 'stdout', '--psm', '11'], stderr=subprocess.DEVNULL, text=True)
    # Small muted status text is lost when Tesseract segments the whole pane.
    # Keep this tighter, contrast-adjusted crop as evidence as well.
    status = out / (name + '-status.png')
    status_width = min(500, width - 260)
    subprocess.run(['ffmpeg', '-hide_banner', '-loglevel', 'error', '-y', '-i', str(path),
                    '-vf', f'crop={status_width}:120:{x+260}:{y+90},scale={status_width*4}:480,eq=contrast=2:brightness=-0.3',
                    '-frames:v', '1', str(status)], check=True)
    text += '\n' + subprocess.check_output(['tesseract', str(status), 'stdout', '--psm', '6'], stderr=subprocess.DEVNULL, text=True)
    if 'result' in name:
        dialog = out / (name + '-dialog.png')
        subprocess.run(['ffmpeg', '-hide_banner', '-loglevel', 'error', '-y', '-i', str(path),
                        '-vf', f'crop=380:160:{x+(width-380)//2}:{y+(height-160)//2},scale=1520:640',
                        '-frames:v', '1', str(dialog)], check=True)
        text += '\n' + subprocess.check_output(['tesseract', str(dialog), 'stdout', '--psm', '6'], stderr=subprocess.DEVNULL, text=True)
    (out / (name + '.txt')).write_text(text)
    return path, ' '.join(text.lower().split())

def wait_text(name, expected, timeout=12):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        path, text = screenshot(name)
        if expected in text:
            print(f'{name}: {text}')
            return path
        time.sleep(.25)
    raise AssertionError(f'{name}: expected {expected!r}, got {text!r}')

def word_center(image, phrase):
    detail = image.with_name(image.stem + '-detail.png')
    is_dialog = phrase == 'ok'
    if is_dialog:
        detail = image.with_name(image.stem + '-dialog.png')
    data = subprocess.check_output(['tesseract', str(detail), 'stdout', '--psm', '6' if is_dialog else '11', 'tsv'], stderr=subprocess.DEVNULL, text=True)
    rows = list(csv.DictReader(io.StringIO(data), delimiter='\t'))
    lines = {}
    for row in rows:
        if row['level'] != '5' or not row['text'].strip():
            continue
        key = tuple(row[k] for k in ('page_num', 'block_num', 'par_num', 'line_num'))
        lines.setdefault(key, []).append(row)
    words = phrase.split()
    for line in lines.values():
        for start in range(len(line) - len(words) + 1):
            segment = line[start:start + len(words)]
            normalized = [re.sub(r'[^a-z]', '', row['text'].lower()) for row in segment]
            if normalized == words:
                first, last = segment[0], segment[-1]
                window = settings_window()[0]
                window_x, window_y = window['at']
                if is_dialog:
                    window_x += (window['size'][0]-380)//2
                    window_y += (window['size'][1]-160)//2
                scale = 4 if is_dialog else 2
                x = window_x + (int(first['left']) + int(last['left']) + int(last['width'])) // (2*scale)
                y = window_y + (2*int(first['top']) + int(first['height'])) // (2*scale)
                return x, y
    raise AssertionError(f'cannot locate {phrase!r} on {image}')

# Prove the Settings window belongs to the exact app, not a stale guest install.
def settings_window():
    clients = json.loads(subprocess.check_output(['hyprctl', 'clients', '-j']))
    return [c for c in clients if c.get('pid') == pid and 'Settings' in c.get('title', '')]

for _ in range(60):
    if settings_window():
        break
    time.sleep(.2)
assert settings_window(), 'staged package did not open Settings'
(out / 'window.json').write_text(json.dumps(settings_window(), indent=2))
address = settings_window()[0]['address']
# Omarchy's first-boot tips cover the right side of a tiled Settings window.
# Float only the staged app window; leave the shell and other guest clients alone.
subprocess.run(['hyprctl', 'eval', f'hl.dispatch(hl.dsp.window.float({{ action = "set", window = "address:{address}" }}))'], check=True)
time.sleep(.5)
(out / 'floating-window.json').write_text(json.dumps(settings_window(), indent=2))
image = wait_text('settings-initial', 'about')
click(*word_center(image, 'about'))
image = wait_text('about-ready', 'check for update')
assert json.loads(config.read_text())['running']['pid'] == pid
assert Path(f'/proc/{pid}/exe').exists(), 'packaged process exited'

def dismiss():
    image = wait_text('result-before-dismiss', 'update check finished')
    click(*word_center(image, 'ok'))
    wait_text('about-after-dismiss', 'check for update')

def check(mode, expected):
    (fixture / 'mode').write_text(mode)
    (fixture / 'waiting').unlink(missing_ok=True)
    (fixture / 'hold').touch()
    image = wait_text(f'{mode}-before-click', 'check for update')
    click(*word_center(image, 'check for'))
    deadline = time.monotonic() + 6
    while time.monotonic() < deadline and not (fixture / 'waiting').exists():
        time.sleep(.1)
    assert (fixture / 'waiting').exists(), f'{mode}: update worker never reached fixture'
    wait_text(f'{mode}-checking', 'checking for updates')
    (fixture / 'hold').unlink()
    wait_text(f'{mode}-result', expected, 20)
    dismiss()

# Each result comes from a real click and a real worker; network replies alone are not evidence.
check('no-update', 'no new update is available')
check('available', 'update installed')
sidecar = Path(os.environ['SCRAWL_GUI_BINARY']).with_suffix('.AppImage.apparent-update.json')
assert json.loads(sidecar.read_text()) == {'channel': 'stable', 'id': 'v99.0.0'}
assert json.loads(config.read_text())['running']['pid'] == pid, 'update restarted the app'
check('error', 'could not check for updates')
(out / 'requests.log').write_text((fixture / 'requests.log').read_text())
(out / 'sidecar.json').write_text(sidecar.read_text())
print('GUI update button: initial, checking, no-update, installed and error visible; installed image stays job-local')
PY
python3 "$GUI_FIXTURE/controller.py" "$SCRAWL_GUI_OUTPUT" "$GUI_FIXTURE" "$APPARENT_CONFIG" "$app_pid"

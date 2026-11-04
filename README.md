# crownos-polkit-agent

The polkit authentication agent of CrownOS. When an app asks polkit for a privileged action
(mounting a drive through UDisks2, `pkexec`, …), polkitd calls this agent, which shows a
CrownUI password prompt over the whole screen.

## How it works

- `crownos-polkit-agent` is the resident daemon. It has no UI and no GPU context; it registers
  `/org/crownos/PolicyKit1/AuthenticationAgent` with
  `org.freedesktop.PolicyKit1.Authority.RegisterAuthenticationAgent` for the logind session
  (`XDG_SESSION_ID`, else the session of its PID, else the user's graphical session).
- For each `BeginAuthentication` it starts `crownos-polkit-dialog` beside it, writes the request
  on its stdin (NUL-separated fields), and replies when the dialog exits: 0 is success, anything
  else is `org.freedesktop.PolicyKit1.Error.Cancelled`. `CancelAuthentication` kills the dialog.
  Requests are shown one at a time, in order.
- The dialog runs the `polkit-agent-helper-1` conversation itself: through
  `/run/polkit/agent-helper.socket` (polkit ≥ 126, socket activated) when present, else by spawning
  the setuid helper. A wrong password shakes the card and starts a fresh attempt.

## Build and install

```sh
CARGO_INCREMENTAL=0 cargo build --release
install -Dm755 target/release/crownos-polkit-agent target/release/crownos-polkit-dialog -t /usr/bin
install -Dm644 data/crownos-polkit-agent.service -t /usr/lib/systemd/user
systemctl --user enable --now crownos-polkit-agent.service
```

The unit is `PartOf=graphical-session.target`, so it starts and stops with the CrownOS session;
the session must import `WAYLAND_DISPLAY` into the systemd user environment
(`systemctl --user import-environment WAYLAND_DISPLAY`). Only one agent may serve a session:
disable any other polkit agent first. `cargo build --no-default-features` builds only the daemon.

## Checks

```sh
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

The tests never talk to the system polkit: the D-Bus interface is tested over a peer-to-peer
connection with stand-in dialog programs, the conversation against a scripted helper, and the
dialog view with `crowntest`.

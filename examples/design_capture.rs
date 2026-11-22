//! Captures every dialog state for the CrownOS-Design Figma file. Run both appearances with
//! `../crownui/capture/scripts/capture-app.sh .`; output lands in `$CROWN_DESIGN_DIR`.

use std::{cell::Cell, rc::Rc};

use capture::{CaptureApp, Carrier};
use crownos_polkit_agent::{
    auth::Echo,
    dialog::{DialogState, authenticator::Update, shell},
};
use crownui::{kit::provide_configured_theme, prelude::component};

const OUTPUT: (f32, f32) = (1920.0, 1080.0);
const MESSAGE: &str = "Authentication is required to mount Backup";

fn open(users: &[&str]) -> anyhow::Result<(CaptureApp, DialogState)> {
    let users: Vec<String> = users.iter().map(|user| (*user).to_owned()).collect();
    let mounted = Rc::new(Cell::new(None));
    let app = capture::mount(
        component({
            let mounted = Rc::clone(&mounted);
            move |cx| {
                provide_configured_theme(cx);
                let state = DialogState::new(cx, users);
                mounted.set(Some(state));
                shell(cx, state, MESSAGE.to_owned(), |_, _| {})
            }
        }),
        OUTPUT.0,
        OUTPUT.1,
    )?;
    let state = mounted
        .get()
        .ok_or_else(|| anyhow::anyhow!("the dialog did not mount"))?;
    Ok((app, state))
}

fn main() -> anyhow::Result<()> {
    let mut carrier = Carrier::new(
        "Polkit Agent",
        "crownos-polkit-agent: the fullscreen authentication overlay, captured headlessly from the current source at a 1920×1080 output.",
    );

    let (mut app, _) = open(&["alice"])?;
    app.pointer_move(10.0, 10.0);
    carrier.capture_state(
        &mut app,
        "Authentication",
        "Password",
        "One user, the answer field focused",
    )?;

    let (mut app, state) = open(&["alice"])?;
    state.answer.set(app.ui().cx(), "hunter22".to_owned());
    app.settle();
    carrier.capture_state(
        &mut app,
        "Authentication",
        "Typed",
        "Answer entered, Authenticate enabled",
    )?;

    let (mut app, _) = open(&["alice", "root"])?;
    carrier.capture_state(
        &mut app,
        "Authentication",
        "Choose user",
        "Several identities offer a user picker",
    )?;

    let (mut app, state) = open(&["alice"])?;
    state.apply(app.ui().cx(), Update::Rejected);
    app.settle();
    carrier.capture_state(
        &mut app,
        "Authentication",
        "Rejected",
        "Wrong answer: error line after the shake",
    )?;

    let (mut app, state) = open(&["alice"])?;
    state.apply(
        app.ui().cx(),
        Update::Prompt {
            text: "Verification code: ".into(),
            echo: Echo::Visible,
        },
    );
    app.settle();
    carrier.capture_state(
        &mut app,
        "Authentication",
        "Helper prompt",
        "A second factor the PAM helper asks for",
    )?;

    let (mut app, state) = open(&["alice"])?;
    state.apply(
        app.ui().cx(),
        Update::Unavailable("pam: helper is not installed".into()),
    );
    app.settle();
    carrier.capture_state(
        &mut app,
        "Authentication",
        "Unavailable",
        "The helper could not start",
    )?;

    let path = carrier.write(None)?;
    println!("{}", path.display());
    capture::finish()
}

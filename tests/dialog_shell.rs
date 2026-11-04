#![allow(clippy::unwrap_used)]

use std::{
    cell::Cell,
    rc::Rc,
    sync::mpsc::{Receiver, channel},
};

use crownos_polkit_agent::{
    auth::Echo,
    dialog::{DialogState, Intent, authenticator::Update, shell},
};
use crowntest::{Role, TestApp, role};
use crownui::prelude::component;

struct Dialog {
    app: TestApp,
    state: DialogState,
    intents: Receiver<Intent>,
}

impl Dialog {
    fn open(users: &[&str]) -> Self {
        let users: Vec<String> = users.iter().map(|user| (*user).to_owned()).collect();
        let (sender, intents) = channel();
        let mounted = Rc::new(Cell::new(None));
        let app = TestApp::with_size(
            component({
                let mounted = Rc::clone(&mounted);
                move |cx| {
                    let state = DialogState::new(cx, users);
                    mounted.set(Some(state));
                    shell(
                        cx,
                        state,
                        "Authentication is required to mount Backup".to_owned(),
                        move |_, intent| sender.send(intent).unwrap(),
                    )
                }
            }),
            1280.0,
            800.0,
        );
        let state = mounted.get().unwrap();
        Self {
            app,
            state,
            intents,
        }
    }

    fn apply(&mut self, update: Update) {
        let state = self.state;
        state.apply(self.app.ui().cx(), update);
        self.app.settle();
    }

    fn intents(&self) -> Vec<Intent> {
        self.intents.try_iter().collect()
    }
}

#[test]
fn shows_the_request_and_reports_buttons() {
    let mut dialog = Dialog::open(&["alice"]);
    assert!(
        dialog
            .app
            .exists(role(Role::Text).text("Authentication Required"))
    );
    assert!(dialog.app.exists(role(Role::Text).text("mount Backup")));
    assert!(!dialog.app.exists(role(Role::ComboBox)));
    assert!(dialog.app.find(role(Role::TextField)).states.secure);

    dialog.app.click(role(Role::Button).name("Authenticate"));
    dialog.app.click(role(Role::Button).name("Cancel"));
    assert_eq!(dialog.intents(), [Intent::Submit, Intent::Cancel]);
}

#[test]
fn escape_cancels() {
    let mut dialog = Dialog::open(&["alice"]);
    dialog.app.pointer_move(10.0, 10.0).press_key("Escape");
    assert_eq!(dialog.intents(), [Intent::Cancel]);
}

#[test]
fn the_answer_field_takes_focus() {
    let mut dialog = Dialog::open(&["alice"]);
    dialog.app.pointer_move(10.0, 10.0);
    assert!(dialog.app.find(role(Role::TextField)).states.focused);
}

#[test]
fn offers_a_choice_of_users() {
    let mut dialog = Dialog::open(&["alice", "root"]);
    assert!(dialog.app.exists(role(Role::ComboBox)));
    assert!(dialog.app.exists(role(Role::Text).text("Authenticate as")));
}

#[test]
fn a_rejected_answer_shows_an_error_and_shakes() {
    let mut dialog = Dialog::open(&["alice"]);
    let state = dialog.state;
    state.answer.set(dialog.app.ui().cx(), "wrong".to_owned());
    assert_eq!(
        state.take_answer(dialog.app.ui().cx()).as_deref(),
        Some("wrong")
    );
    dialog.app.settle();
    assert!(
        dialog
            .app
            .find(role(Role::Button).name("Authenticate"))
            .states
            .disabled
    );
    assert_eq!(state.take_answer(dialog.app.ui().cx()), None);

    state.apply(dialog.app.ui().cx(), Update::Rejected);
    dialog.app.advance_ms(100);
    assert_ne!(state.shake_offset.get(dialog.app.ui().cx()), 0.0);
    assert!(dialog.app.exists(role(Role::Text).text("didn't work")));
    assert!(
        !dialog
            .app
            .find(role(Role::Button).name("Authenticate"))
            .states
            .disabled
    );
    dialog.app.settle();
    assert_eq!(state.shake_offset.get(dialog.app.ui().cx()), 0.0);
}

#[test]
fn follows_the_helper_prompt() {
    let mut dialog = Dialog::open(&["alice"]);
    dialog.apply(Update::Prompt {
        text: "Verification code: ".into(),
        echo: Echo::Visible,
    });
    assert!(
        dialog
            .app
            .exists(role(Role::Text).text("Verification code"))
    );
    assert!(!dialog.app.find(role(Role::TextField)).states.secure);
    dialog.apply(Update::Info("Touch your security key".into()));
    assert!(dialog.app.exists(role(Role::Text).text("security key")));
}

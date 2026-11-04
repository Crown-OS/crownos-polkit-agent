use std::process;

use crownui::{
    kit::provide_configured_theme,
    prelude::{View, component},
};

use super::{
    DialogState, Intent,
    authenticator::{Command, Update, authenticate},
    shell,
};
use crate::{auth::Helper, request::DialogRequest, util::worker::spawn_worker};

/// The exit status the daemon reads as "dismissed"; 0 means authenticated.
const DISMISSED: i32 = 1;

/// The dialog wired to a live helper conversation for `request`.
pub fn root(request: DialogRequest) -> impl View {
    component(move |cx| {
        provide_configured_theme(cx);
        let DialogRequest {
            message,
            cookie,
            users,
            ..
        } = request;
        let first_user = users.first().cloned().unwrap_or_default();
        let state = DialogState::new(cx, users);
        let start_helper = move |user: String| {
            let cookie = cookie.clone();
            async move { Helper::start(&user, &cookie).await }
        };
        let commands = spawn_worker(
            cx,
            move |updates, commands| authenticate(start_helper, first_user, updates, commands),
            move |cx, update| match update {
                Update::Authenticated => process::exit(0),
                update => state.apply(cx, update),
            },
        );
        shell(cx, state, message, move |cx, intent| {
            let command = match intent {
                Intent::Submit => state.take_answer(cx).map(Command::Answer),
                Intent::SwitchUser => state.switch_user(cx).map(Command::SwitchUser),
                Intent::Cancel => process::exit(DISMISSED),
            };
            if let Some(command) = command {
                _ = commands.send(command);
            }
        })
    })
}

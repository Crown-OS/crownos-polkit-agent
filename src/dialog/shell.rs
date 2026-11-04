use crownui::{
    ext::{Key, NamedKey},
    kit::{self, ButtonOptions, ButtonVariant, TextInputOptions, Theme, card, icons, use_theme},
    prelude::{
        Color, Cx, Event, IntoProp, Listeners, Semantic, Signal, Styled, View, bind, hstack, show,
        spacer, text, vstack,
    },
};

use super::{DialogState, Feedback, autofocus::autofocus};
use crate::auth::Echo;

const CARD_WIDTH: f32 = 380.0;
const SCRIM_ALPHA: f32 = 0.45;
const ANSWER_PLACEHOLDER: &str = "Password";

/// What the user asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Submit,
    Cancel,
    SwitchUser,
}

/// The dialog's view: a dimmed scrim over the whole output and the prompt card centered on it.
/// It only reads `state` and reports intents, so tests drive it without a helper.
pub fn shell<F>(
    cx: &mut Cx,
    state: DialogState,
    message: String,
    on_intent: F,
) -> impl View + use<F>
where
    F: Fn(&mut Cx, Intent) + Clone + 'static,
{
    let theme = use_theme(cx);
    let scrim = bind(move |rt| {
        theme.with(rt, |theme| Color {
            a: SCRIM_ALPHA,
            ..theme.surface.shadow
        })
    });
    let feedback_color = bind(move |rt| {
        let is_error = state.feedback.with(rt, Feedback::is_error);
        theme.with(rt, |theme| {
            if is_error {
                theme.status.danger
            } else {
                theme.text.muted
            }
        })
    });
    let offers_choice = state.users.with(cx, |users| users.len() > 1);
    let cancel = on_intent.clone();

    let header = vstack((
        kit::icon(icons::SHIELD_CHECK).size(32.0),
        text("Authentication Required")
            .text_size(17.0)
            .font_semibold()
            .text_color(theme_color(theme, |theme| theme.text.primary)),
        text(message)
            .text_size(13.0)
            .text_center()
            .text_color(theme_color(theme, |theme| theme.text.muted)),
    ))
    .items_center()
    .gap(8.0);

    let user_picker = offers_choice.then(|| {
        vstack((
            text("Authenticate as")
                .text_size(12.0)
                .text_color(theme_color(theme, |theme| theme.text.body)),
            kit::select(state.users, state.selected_user)
                .on_change(emit(&on_intent, Intent::SwitchUser)),
        ))
        .gap(6.0)
    });

    let answer_field = vstack((
        text(bind(move |rt| state.prompt.get(rt)))
            .text_size(12.0)
            .text_color(theme_color(theme, |theme| theme.text.body)),
        show(
            bind(move |rt| state.echo.get(rt) == Echo::Visible),
            {
                let on_intent = on_intent.clone();
                move || {
                    autofocus(
                        kit::text_input(state.answer)
                            .stretch()
                            .on_submit(emit(&on_intent, Intent::Submit)),
                    )
                    .w_full()
                }
            },
            {
                let on_intent = on_intent.clone();
                move || {
                    autofocus(
                        kit::text_input(state.answer)
                            .placeholder(ANSWER_PLACEHOLDER)
                            .secret()
                            .stretch()
                            .on_submit(emit(&on_intent, Intent::Submit)),
                    )
                    .w_full()
                }
            },
        )
        .w_full(),
        text(bind(move |rt| {
            state
                .feedback
                .with(rt, |feedback| feedback.text().to_owned())
        }))
        .text_size(12.0)
        .text_color(feedback_color)
        .hidden(bind(move |rt| state.feedback.get(rt) == Feedback::None)),
    ))
    .gap(6.0);

    let actions = hstack((
        spacer(),
        kit::button("Cancel")
            .variant(ButtonVariant::Secondary)
            .on_click(emit(&on_intent, Intent::Cancel)),
        kit::button("Authenticate")
            .variant(ButtonVariant::Primary)
            .disabled(bind(move |rt| state.verifying.get(rt)))
            .on_click(emit(&on_intent, Intent::Submit)),
    ))
    .gap(8.0);

    vstack((card(
        vstack((header, user_picker, answer_field, actions))
            .p(24.0)
            .gap(18.0),
    )
    .w(CARD_WIDTH)
    .translate_x(state.shake_offset),))
    .w_full()
    .h_full()
    .items_center()
    .justify_center()
    .bg(scrim)
    .on_key_down(move |cx, event| {
        if event.key == Key::Named(NamedKey::Escape) {
            cancel(cx, Intent::Cancel);
        }
    })
}

fn theme_color(theme: Signal<Theme>, pick: fn(&Theme) -> Color) -> impl IntoProp<Color> {
    bind(move |rt| theme.with(rt, pick))
}

fn emit<T>(
    on_intent: &(impl Fn(&mut Cx, Intent) + Clone + 'static),
    intent: Intent,
) -> impl FnMut(&mut Cx, &mut Event<T>) + 'static {
    let on_intent = on_intent.clone();
    move |cx, _| on_intent(cx, intent)
}

use std::future::Future;

use crownui::prelude::Cx;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

const EVENTS_PER_WAKE: usize = 16;

/// Runs `run` on CrownUI's task runtime for as long as the calling component lives: it receives
/// commands through the returned sender and posts events that `on_event` handles on the UI thread.
pub fn spawn_worker<C, E, F>(
    cx: &mut Cx,
    run: impl FnOnce(UnboundedSender<E>, UnboundedReceiver<C>) -> F,
    on_event: impl FnMut(&mut Cx, E) + 'static,
) -> UnboundedSender<C>
where
    C: Send + 'static,
    E: Send + 'static,
    F: Future<Output = ()> + Send + 'static,
{
    let (commands, command_receiver) = unbounded_channel();
    let (events, event_receiver) = unbounded_channel();
    cx.spawn(run(events, command_receiver), |_, ()| {});
    pump(cx, event_receiver, on_event);
    commands
}

fn pump<E: Send + 'static>(
    cx: &mut Cx,
    mut events: UnboundedReceiver<E>,
    mut on_event: impl FnMut(&mut Cx, E) + 'static,
) {
    let next_events = async move {
        let mut batch = Vec::new();
        let open = events.recv_many(&mut batch, EVENTS_PER_WAKE).await > 0;
        (events, batch, open)
    };
    cx.spawn(next_events, move |cx, (events, batch, open)| {
        for event in batch {
            on_event(cx, event);
        }
        if open {
            pump(cx, events, on_event);
        }
    });
}

use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard, PoisonError},
};

use tokio::sync::{Semaphore, SemaphorePermit, oneshot};

/// Lets one authentication on screen at a time, in arrival order, and cancels any of them by
/// cookie whether it is waiting or showing.
pub struct AuthQueue {
    turn: Semaphore,
    cancellations: Mutex<HashMap<String, oneshot::Sender<()>>>,
}

/// A request's place in the queue; it leaves the queue when dropped.
pub struct Ticket<'queue> {
    queue: &'queue AuthQueue,
    cookie: String,
    cancelled: oneshot::Receiver<()>,
}

impl Default for AuthQueue {
    fn default() -> Self {
        Self {
            turn: Semaphore::new(1),
            cancellations: Mutex::default(),
        }
    }
}

impl AuthQueue {
    pub fn enqueue(&self, cookie: &str) -> Ticket<'_> {
        let (cancel, cancelled) = oneshot::channel();
        self.cancellations().insert(cookie.to_owned(), cancel);
        Ticket {
            queue: self,
            cookie: cookie.to_owned(),
            cancelled,
        }
    }

    /// Cancels the request with `cookie`; false when there is none.
    pub fn cancel(&self, cookie: &str) -> bool {
        self.cancellations()
            .remove(cookie)
            .is_some_and(|cancel| cancel.send(()).is_ok())
    }

    fn cancellations(&self) -> MutexGuard<'_, HashMap<String, oneshot::Sender<()>>> {
        self.cancellations
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl<'queue> Ticket<'queue> {
    /// Waits until no other authentication is on screen; `None` when cancelled first.
    pub async fn wait_turn(&mut self) -> Option<SemaphorePermit<'queue>> {
        tokio::select! {
            permit = self.queue.turn.acquire() => permit.ok(),
            _ = &mut self.cancelled => None,
        }
    }

    /// Resolves once polkitd cancels this request.
    pub async fn cancelled(&mut self) {
        _ = (&mut self.cancelled).await;
    }
}

impl Drop for Ticket<'_> {
    fn drop(&mut self) {
        self.queue.cancellations().remove(&self.cookie);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::timeout;

    use super::*;

    const BRIEFLY: Duration = Duration::from_millis(50);

    #[tokio::test]
    async fn serves_one_request_at_a_time() {
        let queue = AuthQueue::default();
        let mut first = queue.enqueue("first");
        let mut second = queue.enqueue("second");
        let turn = first.wait_turn().await;
        assert!(turn.is_some());
        assert!(timeout(BRIEFLY, second.wait_turn()).await.is_err());
        drop(turn);
        assert!(second.wait_turn().await.is_some());
    }

    #[tokio::test]
    async fn cancels_a_waiting_request() {
        let queue = AuthQueue::default();
        let mut showing = queue.enqueue("showing");
        let _turn = showing.wait_turn().await;
        let mut waiting = queue.enqueue("waiting");
        assert!(queue.cancel("waiting"));
        assert!(waiting.wait_turn().await.is_none());
    }

    #[tokio::test]
    async fn cancels_a_showing_request() {
        let queue = AuthQueue::default();
        let mut showing = queue.enqueue("showing");
        let _turn = showing.wait_turn().await;
        assert!(queue.cancel("showing"));
        timeout(BRIEFLY, showing.cancelled()).await.unwrap();
    }

    #[test]
    fn forgets_dropped_requests() {
        let queue = AuthQueue::default();
        drop(queue.enqueue("gone"));
        assert!(!queue.cancel("gone"));
    }
}

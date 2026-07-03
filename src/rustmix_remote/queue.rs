//! Small thread-safe queue used as the BLE callback boundary.
//!
//! The BLE GATT write callback pushes `RemoteEvent`s here. The firmware main
//! loop drains the queue and applies the event through the normal UI path.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use super::RemoteEvent;

pub const REMOTE_EVENT_QUEUE_CAPACITY: usize = 8;

#[derive(Clone, Debug)]
pub struct RemoteEventQueue {
    inner: Arc<Mutex<VecDeque<RemoteEvent>>>,
    capacity: usize,
}

impl Default for RemoteEventQueue {
    fn default() -> Self {
        Self::new(REMOTE_EVENT_QUEUE_CAPACITY)
    }
}

impl RemoteEventQueue {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::with_capacity(capacity.max(1)))),
            capacity: capacity.max(1),
        }
    }

    /// Push an event from a producer such as the BLE GATT callback.
    ///
    /// If the queue is full, the oldest event is dropped. For page turning,
    /// keeping the newest command is safer than allowing an old burst to replay.
    pub fn push(&self, event: RemoteEvent) {
        let mut inner = self.inner.lock().unwrap();
        if inner.len() >= self.capacity {
            inner.pop_front();
        }
        inner.push_back(event);
    }

    pub fn pop(&self) -> Option<RemoteEvent> {
        self.inner.lock().unwrap().pop_front()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&self) {
        self.inner.lock().unwrap().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_returns_events_in_order() {
        let queue = RemoteEventQueue::new(4);
        queue.push(RemoteEvent::PageNext);
        queue.push(RemoteEvent::PagePrevious);
        assert_eq!(queue.pop(), Some(RemoteEvent::PageNext));
        assert_eq!(queue.pop(), Some(RemoteEvent::PagePrevious));
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn queue_drops_oldest_when_full() {
        let queue = RemoteEventQueue::new(2);
        queue.push(RemoteEvent::PageNext);
        queue.push(RemoteEvent::PagePrevious);
        queue.push(RemoteEvent::Menu);
        assert_eq!(queue.pop(), Some(RemoteEvent::PagePrevious));
        assert_eq!(queue.pop(), Some(RemoteEvent::Menu));
        assert_eq!(queue.pop(), None);
    }
}

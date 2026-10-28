//! A bounded worker for replaceable, read-only previews. Never use it for writes.
use std::sync::{Arc, Condvar, Mutex};
struct State<Q, R> {
    generation: u64,
    pending: Option<(u64, Q)>,
    result: Option<R>,
    stopped: bool,
}
struct Shared<Q, R> {
    state: Mutex<State<Q, R>>,
    wake: Condvar,
}
pub struct LatestRead<Q, R> {
    shared: Arc<Shared<Q, R>>,
}
impl<Q: Send + 'static, R: Send + 'static> LatestRead<Q, R> {
    pub fn new(mut read: impl FnMut(Q) -> R + Send + 'static) -> Self {
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                generation: 0,
                pending: None,
                result: None,
                stopped: false,
            }),
            wake: Condvar::new(),
        });
        let worker = shared.clone();
        std::thread::spawn(move || {
            loop {
                let request = {
                    let mut state = worker.state.lock().unwrap();
                    while state.pending.is_none() && !state.stopped {
                        state = worker.wake.wait(state).unwrap();
                    }
                    if state.stopped {
                        return;
                    }
                    state.pending.take().unwrap()
                };
                let result = read(request.1);
                let mut state = worker.state.lock().unwrap();
                if state.stopped {
                    return;
                }
                if state.generation == request.0 {
                    state.result = Some(result);
                }
            }
        });
        Self { shared }
    }
    pub fn request(&self, request: Q) {
        let mut state = self.shared.state.lock().unwrap();
        state.generation = state.generation.wrapping_add(1);
        state.pending = Some((state.generation, request));
        state.result = None;
        self.shared.wake.notify_one();
    }
    pub fn cancel(&self) {
        let mut state = self.shared.state.lock().unwrap();
        state.generation = state.generation.wrapping_add(1);
        state.pending = None;
        state.result = None;
    }
    pub fn take_result(&self) -> Option<R> {
        self.shared.state.lock().unwrap().result.take()
    }
}
impl<Q, R> Drop for LatestRead<Q, R> {
    fn drop(&mut self) {
        let mut state = self.shared.state.lock().unwrap();
        state.stopped = true;
        state.pending = None;
        state.result = None;
        self.shared.wake.notify_one();
    }
}

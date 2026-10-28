use gitview::read_worker::LatestRead;
use std::{sync::mpsc, time::Duration};
#[test]
fn coalesces_pending_reads_and_discards_cancelled_completions() {
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let worker = LatestRead::new(move |request: u32| {
        started_tx.send(request).unwrap();
        if request == 1 {
            release_rx.recv().unwrap();
        }
        request * 10
    });
    worker.request(1);
    assert_eq!(started_rx.recv_timeout(Duration::from_secs(1)).unwrap(), 1);
    worker.request(2);
    worker.request(3);
    worker.cancel();
    worker.request(4);
    release_tx.send(()).unwrap();
    assert_eq!(started_rx.recv_timeout(Duration::from_secs(1)).unwrap(), 4);
    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    let result = loop {
        if let Some(result) = worker.take_result() {
            break result;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    };
    assert_eq!(result, 40);
    assert!(started_rx.try_recv().is_err());
}

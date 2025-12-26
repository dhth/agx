use crate::domain::DebugEventReceiver;
use std::time::Duration;

pub struct DebugServer {
    debug_rx: DebugEventReceiver,
}

impl DebugServer {
    pub fn new(events_rx: DebugEventReceiver) -> Self {
        Self {
            debug_rx: events_rx,
        }
    }

    pub async fn run(&self) {
        let mut rx = self.debug_rx.subscribe();

        let mut count = 0;
        loop {
            match rx.recv().await {
                Ok(e) => {
                    println!("debugger got event: {}", e.kind());
                    count += 1;
                }
                Err(e) => {
                    eprintln!("debugger couldn't receive event: {e}");
                }
            }

            if count >= 3 {
                break;
            }
        }

        println!("debugger available at http://127.0.0.1/debugger");
        tokio::time::sleep(Duration::from_secs(10)).await;
        println!("debugger has shut down");
    }
}

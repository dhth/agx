use tokio::sync::watch;

#[derive(Clone)]
pub struct CancelTx(watch::Sender<bool>);

impl CancelTx {
    pub fn cancel(&self) -> bool {
        self.0.send(true).is_ok()
    }

    pub fn is_cancelled(&self) -> bool {
        *self.0.borrow()
    }

    pub fn reset(&self) -> bool {
        self.0.send(false).is_ok()
    }
}

#[derive(Clone)]
pub struct CancelRx(watch::Receiver<bool>);

impl CancelRx {
    pub async fn wait_for_cancellation(&mut self) {
        if *self.0.borrow() {
            return;
        }

        while self.0.changed().await.is_ok() {
            if *self.0.borrow() {
                return;
            }
        }
    }
}

pub fn cancellation_channel() -> (CancelTx, CancelRx) {
    let (tx, rx) = watch::channel(false);
    (CancelTx(tx), CancelRx(rx))
}

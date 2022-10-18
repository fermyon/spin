use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

const SLOW_UPLOAD_WARNING_DELAY_MILLIS: u64 = 2500;

pub(crate) struct SlothWarning<T> {
    warning: JoinHandle<T>,
}

impl<T> Drop for SlothWarning<T> {
    fn drop(&mut self) {
        self.warning.abort()
    }
}

pub(crate) fn warn_if_slow_response(message: impl Into<String>) -> SlothWarning<()> {
    let message = message.into();
    let warning = tokio::spawn(warn_slow_response(message));
    SlothWarning { warning }
}

async fn warn_slow_response(message: String) {
    sleep(Duration::from_millis(SLOW_UPLOAD_WARNING_DELAY_MILLIS)).await;
    eprintln!("{}", message);
    eprintln!();
}

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

pub(crate) fn warn_if_slow_response(url: &str) -> SlothWarning<()> {
    let url = url.to_owned();
    let warning = tokio::spawn(warn_slow_response(url));
    SlothWarning { warning }
}

async fn warn_slow_response(url: String) {
    sleep(Duration::from_millis(SLOW_UPLOAD_WARNING_DELAY_MILLIS)).await;
    eprintln!("{} is responding slowly or not responding...", url);
    eprintln!();
}

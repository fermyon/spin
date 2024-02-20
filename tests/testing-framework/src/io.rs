use std::io::Read;

/// Helper for reading from a child process stream in a non-blocking way
pub struct OutputStream {
    rx: std::sync::mpsc::Receiver<Result<Vec<u8>, std::io::Error>>,
    buffer: Vec<u8>,
}

impl OutputStream {
    pub fn new<R: Read + Send + 'static>(mut stream: R) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut buffer = vec![0; 1024];
            loop {
                let msg = stream.read(&mut buffer).map(|n| buffer[..n].to_vec());
                if let Err(e) = tx.send(msg) {
                    if let Err(e) = e.0 {
                        eprintln!("Error reading from stream: {e}");
                    }
                    break;
                }
            }
        });
        Self {
            rx,
            buffer: Vec::new(),
        }
    }

    /// Get the output of the stream so far
    pub fn output(&mut self) -> &[u8] {
        while let Ok(Ok(s)) = self.rx.try_recv() {
            self.buffer.extend(s);
        }
        &self.buffer
    }

    /// Get the output of the stream so far
    ///
    /// Returns None if the output is not valid utf8
    pub fn output_as_str(&mut self) -> Option<&str> {
        std::str::from_utf8(self.output()).ok()
    }
}

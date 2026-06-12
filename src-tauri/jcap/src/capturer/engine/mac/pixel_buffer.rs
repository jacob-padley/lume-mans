use cidre::{arc, cm, sc};
use std::sync::mpsc;

use crate::capturer::Capturer;

impl Capturer {
    pub fn get_next_sample_buffer(
        &self,
    ) -> Result<(arc::R<cm::SampleBuf>, sc::stream::OutputType), mpsc::RecvError> {
        use std::time::Duration;

        loop {
            let error_flag = self
                .engine
                .error_flag
                .load(std::sync::atomic::Ordering::Relaxed);
            if error_flag {
                return Err(mpsc::RecvError);
            }

            return match self.rx.recv_timeout(Duration::from_millis(10)) {
                Ok(v) => Ok(v),
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => Err(mpsc::RecvError),
            };
        }
    }
}

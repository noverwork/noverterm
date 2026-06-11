use std::collections::HashMap;

use tokio::sync::Mutex;

use crate::runtime::sftp::TransferCancellation;

#[derive(Default)]
pub struct TransferState {
    pub cancellations: Mutex<HashMap<String, TransferCancellation>>,
}

impl TransferState {
    pub fn new() -> Self {
        Self::default()
    }
}

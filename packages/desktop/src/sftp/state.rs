use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::runtime::sftp::TransferCancellation;

#[derive(Default)]
pub struct TransferState {
    pub cancellations: Arc<Mutex<HashMap<String, TransferCancellation>>>,
}

impl TransferState {
    pub fn new() -> Self {
        Self::default()
    }
}

use std::sync::Arc;

use crate::{MyRaft, NodeId};
use crate::Store;

// Representation of an application state. This struct can be shared around to share
// instances of raft, store and more.
pub struct App {
    pub id: NodeId,
    pub addr: String,
    pub raft: MyRaft,
    pub store: Arc<Store>,
    pub config: Arc<openraft::Config>,
}

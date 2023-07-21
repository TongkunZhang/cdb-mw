use std::collections::BTreeMap;
use std::fmt::Display;
use std::path::Path;
use std::sync::Arc;

use openraft::error::CheckIsLeaderError;
use openraft::raft::ClientWriteResponse;
use openraft::Config;
use openraft::Raft;

use crate::app::App;
use crate::raft_network_impl::NetworkFactory;
use crate::store::Request;
use crate::store::Response;
use crate::store::Store;

mod app;
pub mod raft;
mod raft_network_impl;
mod store;

pub type NodeId = u64;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Default)]
pub struct Node {
    pub grpc_addr: String,
}

impl Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Node {{ grpc_addr: {}}}", self.grpc_addr)
    }
}
openraft::declare_raft_types!(
    /// Declare the type configuration for example K/V store.
    pub TypeConfig: D = Request, R = Response, NodeId = NodeId, Node = Node
);

pub type MyRaft = Raft<TypeConfig, NetworkFactory, Arc<Store>>;

pub struct KVStore {
    pub app: Arc<App>,
}

impl KVStore {
    pub async fn new<P>(id: NodeId, dir: P, addr: String) -> Self
    where
        P: AsRef<Path>,
    {
        let config = Config {
            heartbeat_interval: 250,
            election_timeout_min: 299,
            ..Default::default()
        };
        let config = Arc::new(config.validate().unwrap());
        let store = Store::new(&dir).await;
        let network = NetworkFactory {};
        let raft = MyRaft::new(id, config.clone(), network, store.clone())
            .await
            .unwrap();

        let app = Arc::new(App {
            id,
            addr,
            raft,
            store,
            config,
        });

        KVStore { app }
    }

    pub async fn start(&self, existing_node: Option<(NodeId, Node)>) {
        if let Some(existing_node) = existing_node {
            self.app
                .raft
                .add_learner(existing_node.0, existing_node.1, true)
                .await
                .unwrap();
        } else {
            let mut nodes = BTreeMap::new();
            let node = Node {
                grpc_addr: self.app.addr.clone(),
            };
            nodes.insert(self.app.id, node);
            self.app.raft.initialize(nodes).await.unwrap();
        }
    }

    pub async fn write(&self, key: String, value: String) -> ClientWriteResponse<TypeConfig> {
        self.app
            .raft
            .client_write(Request::Set { key, value })
            .await
            .unwrap()
    }

    pub async fn read(&self, key: &str) -> anyhow::Result<String> {
        let state_machine = self.app.store.state_machine.read().await;
        let value = state_machine.get(&key)?;

        Ok(value.unwrap_or_default())
    }

    pub async fn consistent_read(&self, key: &str) -> anyhow::Result<String> {
        self.app.raft.is_leader().await?;

        let state_machine = self.app.store.state_machine.read().await;
        let value = state_machine.get(&key)?;

        let res: Result<String, CheckIsLeaderError<NodeId, Node>> = Ok(value.unwrap_or_default());

        Ok(res?)
    }
}

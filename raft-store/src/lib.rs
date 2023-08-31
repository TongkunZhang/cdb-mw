use std::collections::BTreeMap;
use std::fmt::Display;
use std::path::Path;
use std::sync::Arc;

use openraft::error::CheckIsLeaderError;
use openraft::raft::ClientWriteResponse;
use openraft::Config;
use openraft::Raft;

use crate::app::App;
use crate::raft::raft_proto::raft_client::RaftClient;
use crate::raft::raft_proto::RaftRequest;
use crate::raft_network_impl::NetworkFactory;
use crate::store::Request;
use crate::store::Response;
use crate::store::Store;
use tracing::{debug, error, info};
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
    pub async fn new<P>(id: NodeId, dir: P, addr: String, new_cluster: bool) -> Self
    where
        P: AsRef<Path>,
    {
        let config = Config {
            heartbeat_interval: 500,
            election_timeout_min: 1500,
            election_timeout_max: 3000,
            ..Default::default()
        };
        let config = Arc::new(config.validate().unwrap());
        let store = Store::new(&dir).await;
        let network = NetworkFactory::new();
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

        if new_cluster {
            let mut nodes = BTreeMap::new();
            let node = Node {
                grpc_addr: app.addr.clone(),
            };
            nodes.insert(app.id, node);
            let result = app.raft.initialize(nodes).await;
            if let Err(e) = result {
                error!("Failed to initialize cluster: {:?}", e);
            }
        }
        info!("KVStore initialized with ID: {}", id);

        KVStore { app }
    }

    pub async fn write(&self, key: &str, value: &str) -> ClientWriteResponse<TypeConfig> {
        debug!("Initiating write operation for key: {}", key);

        let req = Request::Set {
            key: key.to_string(),
            value: value.to_string(),
        };
        let metrics_object = self.app.raft.metrics();
        let metrics = metrics_object.borrow().clone();

        if let Some(current_leader) = metrics.current_leader {
            if metrics.id == current_leader {
                match self.app.raft.client_write(req).await {
                    Ok(response) => {
                        info!("Write operation succeeded for key: {}", key);
                        response
                    }
                    Err(e) => {
                        error!("Unexpected error during write: {:?}", e);
                        panic!("Unexpected error: {:?}", e);
                    }
                }
            } else {
                let members = metrics
                    .membership_config
                    .membership()
                    .get_node(&current_leader);
                match members {
                    Some(node) => {
                        let addr = format!("http://{}", node.grpc_addr);
                        let mut client = RaftClient::connect(addr.clone())
                            .await
                            .expect("Failed to connect");
                        let resp = client
                            .forward(RaftRequest {
                                data: serde_json::to_string(&req).expect("fail to serialize req"),
                            })
                            .await
                            .expect("Failed to forward");
                        info!("Forwarded write request to leader for key: {}", key);
                        let mes = resp.into_inner();
                        let resp: ClientWriteResponse<TypeConfig> =
                            serde_json::from_str(&mes.data).unwrap();
                        resp
                    }
                    None => {
                        error!("No leader found during write operation");
                        panic!("No leader");
                    }
                }
            }
        } else {
            error!("No leader found during write operation");
            panic!("No leader");
        }
    }

    pub async fn read(&self, key: &str) -> anyhow::Result<String> {
        debug!("Initiating read operation for key: {}", key);
        let state_machine = self.app.store.state_machine.read().await;
        let value = state_machine.get(&key)?;

        info!("Read operation completed for key: {}", key);

        Ok(value.unwrap_or_default())
    }

    pub async fn consistent_read(&self, key: &str) -> anyhow::Result<String> {
        debug!("Initiating consistent read for key: {}", key);

        self.app.raft.is_leader().await?;

        let state_machine = self.app.store.state_machine.read().await;
        let value = state_machine.get(&key)?;

        let res: Result<String, CheckIsLeaderError<NodeId, Node>> = Ok(value.unwrap_or_default());
        info!("Consistent read completed for key: {}", key);
        Ok(res?)
    }
}

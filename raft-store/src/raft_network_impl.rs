use crate::raft::raft_proto::raft_client::RaftClient;
use crate::raft::raft_proto::RaftRequest;
use crate::{Node, NodeId, TypeConfig};
use async_trait::async_trait;
use openraft::error::{InstallSnapshotError, NetworkError, RPCError, RaftError};
use openraft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse,
    VoteRequest, VoteResponse,
};
use openraft::{RaftNetwork, RaftNetworkFactory};
use tonic::transport::channel::Channel;
use tracing::{debug, info};

pub struct NetworkFactory {}

impl NetworkFactory {
    pub fn new() -> Self {
        info!("New NetworkFactory instance created");
        NetworkFactory {}
    }
}

#[async_trait]
impl RaftNetworkFactory<TypeConfig> for NetworkFactory {
    type Network = NetworkConnection;
    #[tracing::instrument(level = "debug", skip_all)]
    async fn new_client(&mut self, _target: NodeId, node: &Node) -> NetworkConnection {
        let addr = format!("http://{}", node.grpc_addr);
        info!("New client created for address: {}", addr);
        // if !self.connections.contains_key(&addr) {
        //     let client = RaftClient::connect(addr.clone()).await.expect("Failed to connect");
        //     self.connections.insert(addr.clone(), client);
        // }

        // let client = self.connections.get(&addr).expect("Client should be present").clone();

        NetworkConnection {
            addr,
            // client,
        }
    }
}

pub struct NetworkConnection {
    addr: String,
    // client: RaftClient<Channel>,
}

impl NetworkConnection {
    pub async fn make_client(&self) -> RaftClient<Channel> {
        let client = RaftClient::connect(self.addr.clone())
            .await
            .expect("Failed to connect");

        info!("RaftClient successfully connected to {}", self.addr);
        client
    }
}

#[async_trait]
impl RaftNetwork<TypeConfig> for NetworkConnection {
    #[tracing::instrument(level = "debug", skip_all, err(Debug))]
    async fn send_append_entries(
        &mut self,
        rpc: AppendEntriesRequest<TypeConfig>,
    ) -> Result<AppendEntriesResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {
        let req = RaftRequest {
            data: serde_json::to_string(&rpc)
                .map_err(|e| RPCError::Network(NetworkError::new(&e)))?,
        };
        debug!("Sending AppendEntries RPC with request: {:?}", req);
        let resp = self.make_client().await.append_entries(req).await;
        match &resp {
            Ok(_) => debug!("AppendEntries RPC successfully sent"), // Added log
            Err(e) => debug!("Failed to send AppendEntries RPC: {:?}", e), // Added log
        }

        let resp = resp.map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
        let mes = resp.into_inner();
        let resp = serde_json::from_str(&mes.data)
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        Ok(resp)
    }

    #[tracing::instrument(level = "debug", skip_all, err(Debug))]
    async fn send_install_snapshot(
        &mut self,
        rpc: InstallSnapshotRequest<TypeConfig>,
    ) -> Result<
        InstallSnapshotResponse<NodeId>,
        RPCError<NodeId, Node, RaftError<NodeId, InstallSnapshotError>>,
    > {
        let req = RaftRequest {
            data: serde_json::to_string(&rpc)
                .map_err(|e| RPCError::Network(NetworkError::new(&e)))?,
        };
        debug!("Sending InstallSnapshot RPC with request: {:?}", req);
        let resp = self.make_client().await.install_snapshot(req).await;
        match &resp {
            Ok(_) => debug!("InstallSnapshot RPC successfully sent"),
            Err(e) => debug!("Failed to send InstallSnapshot RPC: {:?}", e),
        }

        let resp = resp.map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
        let mes = resp.into_inner();
        let resp = serde_json::from_str(&mes.data)
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        Ok(resp)
    }

    #[tracing::instrument(level = "debug", skip_all, err(Debug))]
    async fn send_vote(
        &mut self,
        rpc: VoteRequest<NodeId>,
    ) -> Result<VoteResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {
        let req = RaftRequest {
            data: serde_json::to_string(&rpc)
                .map_err(|e| RPCError::Network(NetworkError::new(&e)))?,
        };
        debug!("Sending Vote RPC with request: {:?}", req);

        let resp = self.make_client().await.vote(req).await;
        match &resp {
            Ok(_) => debug!("Vote RPC successfully sent"), // Added log
            Err(e) => debug!("Failed to send Vote RPC: {:?}", e), // Added log
        }

        let resp = resp.map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
        let mes = resp.into_inner();
        let resp = serde_json::from_str(&mes.data)
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        Ok(resp)
    }
}

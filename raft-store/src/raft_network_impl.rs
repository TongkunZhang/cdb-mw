use async_trait::async_trait;
use openraft::{RaftNetwork, RaftNetworkFactory};
use openraft::error::{InstallSnapshotError, NetworkError, RaftError, RPCError};
use openraft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse,
    VoteRequest, VoteResponse,
};
use tonic::transport::channel::Channel;

use crate::{Node, NodeId, TypeConfig};
use crate::raft::raft_proto::raft_client::RaftClient;
use crate::raft::raft_proto::RaftRequest;

pub struct NetworkFactory {}

impl NetworkFactory {
    pub fn new() -> Self {
        println!("new network factory created");
        NetworkFactory {}
    }
}


#[async_trait]
impl RaftNetworkFactory<TypeConfig> for NetworkFactory {
    type Network = NetworkConnection;

    async fn new_client(&mut self, _target: NodeId, node: &Node) -> NetworkConnection {
        let addr = format!("http://{}", node.grpc_addr);
        println!("new client created for {}", addr);

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
        let client = RaftClient::connect(self.addr.clone()).await.expect("Failed to connect");

        client
    }
}

#[async_trait]
impl RaftNetwork<TypeConfig> for NetworkConnection {
    async fn send_append_entries(
        &mut self,
        rpc: AppendEntriesRequest<TypeConfig>,
    ) -> Result<AppendEntriesResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {
        let req = RaftRequest {
            data: serde_json::to_string(&rpc)
                .map_err(|e| RPCError::Network(NetworkError::new(&e)))?,
        };
        let resp = self.make_client().await.append_entries(req).await;

        let resp = resp.map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
        let mes = resp.into_inner();
        let resp = serde_json::from_str(&mes.data)
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        Ok(resp)
    }

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
        let resp = self.make_client().await.install_snapshot(req).await;

        let resp = resp.map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
        let mes = resp.into_inner();
        let resp = serde_json::from_str(&mes.data)
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        Ok(resp)
    }

    async fn send_vote(
        &mut self,
        rpc: VoteRequest<NodeId>,
    ) -> Result<VoteResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {
        let req = RaftRequest {
            data: serde_json::to_string(&rpc)
                .map_err(|e| RPCError::Network(NetworkError::new(&e)))?,
        };
        let resp = self.make_client().await.vote(req).await;

        let resp = resp.map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
        let mes = resp.into_inner();
        let resp = serde_json::from_str(&mes.data)
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        Ok(resp)
    }
}

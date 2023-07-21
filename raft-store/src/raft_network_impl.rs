use async_trait::async_trait;
use openraft::error::{InstallSnapshotError, NetworkError, RPCError, RaftError};
use openraft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse,
    VoteRequest, VoteResponse,
};
use openraft::{RaftNetwork, RaftNetworkFactory};
use tonic::transport::channel::Channel;

use crate::raft::raft_proto::raft_client::RaftClient;
use crate::raft::raft_proto::RaftRequest;
use crate::{Node, NodeId, TypeConfig};

pub struct NetworkFactory {
    // TODO: use a connection pool
    // conn_pool: Pool<ChannelManager>,
}

#[async_trait]
impl RaftNetworkFactory<TypeConfig> for NetworkFactory {
    type Network = Network;

    async fn new_client(&mut self, _target: NodeId, node: &Node) -> Network {
        let addr = format!("http://{}", node.grpc_addr);
        let client = RaftClient::connect(addr.clone()).await.unwrap();
        Network {
            // addr,
            client,
            // _target,
        }
    }
}

pub struct Network {
    // addr: String,
    client: RaftClient<Channel>,
    // target: NodeId,
}

#[async_trait]
impl RaftNetwork<TypeConfig> for Network {
    async fn send_append_entries(
        &mut self,
        rpc: AppendEntriesRequest<TypeConfig>,
    ) -> Result<AppendEntriesResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {
        let req = RaftRequest {
            data: serde_json::to_string(&rpc)
                .map_err(|e| RPCError::Network(NetworkError::new(&e)))?,
        };
        let resp = self.client.append_entries(req).await;

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
        let resp = self.client.install_snapshot(req).await;

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
        let resp = self.client.vote(req).await;

        let resp = resp.map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
        let mes = resp.into_inner();
        let resp = serde_json::from_str(&mes.data)
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        Ok(resp)
    }
}

use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::app::App;
use crate::raft::raft_proto::raft_client::RaftClient;
use crate::raft::raft_proto::raft_server::Raft as RaftService;
use crate::raft::raft_proto::{JoinReply, JoinRequest, PrepareRequest, RaftReply, RaftRequest};
use crate::Node;
use tracing::{error, info, instrument, trace};

pub mod raft_proto {
    tonic::include_proto!("raft_proto");
}

pub struct Raft {
    app: Arc<App>,
}

impl Raft {
    pub fn create(app: Arc<App>) -> Self {
        Self { app }
    }
}

#[async_trait::async_trait]
impl RaftService for Raft {
    #[instrument(skip(self))]
    async fn append_entries(
        &self,
        request: Request<RaftRequest>,
    ) -> Result<Response<RaftReply>, Status> {
        trace!("Received Append Entries request");
        let req = request.into_inner();

        let ae_req =
            serde_json::from_str(&req.data).map_err(|x| Status::internal(x.to_string()))?;

        let resp = self
            .app
            .raft
            .append_entries(ae_req)
            .await
            .map_err(|x| Status::internal(x.to_string()))?;
        let data = serde_json::to_string(&resp).expect("fail to serialize resp");
        let mes = RaftReply {
            data,
            error: "".to_string(),
        };

        Ok(Response::new(mes))
    }

    #[instrument(skip(self))]
    async fn install_snapshot(
        &self,
        request: Request<RaftRequest>,
    ) -> Result<Response<RaftReply>, Status> {
        trace!("Received Install Snapshot request");
        let req = request.into_inner();

        let is_req =
            serde_json::from_str(&req.data).map_err(|x| Status::internal(x.to_string()))?;

        let resp = self
            .app
            .raft
            .install_snapshot(is_req)
            .await
            .map_err(|x| Status::internal(x.to_string()))?;
        let data = serde_json::to_string(&resp).expect("fail to serialize resp");
        let mes = RaftReply {
            data,
            error: "".to_string(),
        };

        Ok(Response::new(mes))
    }

    #[instrument(skip(self))]
    async fn vote(&self, request: Request<RaftRequest>) -> Result<Response<RaftReply>, Status> {
        trace!("Received Vote request");
        let req = request.into_inner();

        let v_req = serde_json::from_str(&req.data).map_err(|x| Status::internal(x.to_string()))?;

        let resp = self
            .app
            .raft
            .vote(v_req)
            .await
            .map_err(|x| Status::internal(x.to_string()))?;
        let data = serde_json::to_string(&resp).expect("fail to serialize resp");
        let mes = RaftReply {
            data,
            error: "".to_string(),
        };

        Ok(Response::new(mes))
    }

    #[instrument(skip(self))]
    async fn forward(&self, request: Request<RaftRequest>) -> Result<Response<RaftReply>, Status> {
        trace!("Received forward request");
        let req = request.into_inner();

        let v_req = serde_json::from_str(&req.data).map_err(|x| Status::internal(x.to_string()))?;

        let resp = self
            .app
            .raft
            .client_write(v_req)
            .await
            .map_err(|x| Status::internal(x.to_string()))?;
        let data = serde_json::to_string(&resp).expect("fail to serialize resp");
        let mes = RaftReply {
            data,
            error: "".to_string(),
        };

        Ok(Response::new(mes))
    }

    #[instrument(skip(self))]
    async fn prepare(
        &self,
        _request: Request<PrepareRequest>,
    ) -> Result<Response<JoinReply>, Status> {
        trace!("Received Prepare request");
        let metrics = self.app.raft.metrics();
        let metrics = metrics.borrow();

        let nodes = metrics.membership_config.nodes();
        let node_id = nodes.max_by_key(|x| x.0).unwrap().0 + 1;

        info!("Prepare: Assigning new node_id: {}", node_id);
        Ok(Response::new(JoinReply {
            error: "".to_string(),
            node_id,
        }))
    }

    #[instrument(skip(self))]
    async fn join(&self, request: Request<JoinRequest>) -> Result<Response<JoinReply>, Status> {
        trace!("Received Join request");
        let metrics_object = self.app.raft.metrics();
        let metrics = metrics_object.borrow().clone();
        if let Some(current_leader) = metrics.current_leader {
            if metrics.id != current_leader {
                info!("Join: Not a leader, forwarding to leader");

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
                            .join(request.into_inner())
                            .await
                            .expect("Failed to forward");
                        info!("Forwarded join request to leader");
                        return Ok(resp);
                    }
                    None => {
                        error!("No leader found during join operation");
                        return Ok(Response::new(JoinReply {
                            error: "no leader found".to_string(),
                            node_id: 0,
                        }));
                    }
                }
            } else {
                let req = request.into_inner();
                let existing_voters = self
                    .app
                    .raft
                    .metrics()
                    .borrow()
                    .membership_config
                    .membership()
                    .voter_ids();
                let mut new_voters: Vec<u64> = Vec::new();
                for i in existing_voters {
                    new_voters.push(i);
                }
                new_voters.push(req.node_id);
                match self
                    .app
                    .raft
                    .add_learner(
                        req.node_id,
                        Node {
                            grpc_addr: req.addr,
                        },
                        true,
                    )
                    .await
                {
                    Ok(_) => {
                        info!("Join: Adding node_id: {}", req.node_id);

                        if let Ok(_) = self.app.raft.change_membership(new_voters, false).await {
                            Ok(Response::new(JoinReply {
                                error: "".to_string(),
                                node_id: req.node_id,
                            }))
                        } else {
                            error!("Failed to change membership");
                            Ok(Response::new(JoinReply {
                                error: "failed to change membership".to_string(),
                                node_id: req.node_id,
                            }))
                        }
                    }
                    Err(e) => {
                        error!("Failed to add learner: {:?}", e);
                        Ok(Response::new(JoinReply {
                            error: e.to_string(),
                            node_id: req.node_id,
                        }))
                    }
                }
            }
        } else {
            error!("No leader found during join operation");
            return Ok(Response::new(JoinReply {
                error: "no leader found".to_string(),
                node_id: 0,
            }));
        }
    }
}

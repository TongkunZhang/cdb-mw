use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::app::App;
use crate::raft::raft_proto::raft_server::Raft as RaftService;
use crate::raft::raft_proto::{RaftReply, RaftRequest};

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
    async fn append_entries(
        &self,
        request: Request<RaftRequest>,
    ) -> Result<Response<RaftReply>, Status> {
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

    async fn install_snapshot(
        &self,
        request: Request<RaftRequest>,
    ) -> Result<Response<RaftReply>, Status> {
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

    async fn vote(&self, request: Request<RaftRequest>) -> Result<Response<RaftReply>, Status> {
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
}

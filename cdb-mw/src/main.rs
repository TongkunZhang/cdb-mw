use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use clap::ArgGroup;
use clap::Parser;
use tonic::transport::Server;

use raft_store::KVStore;
use raft_store::raft::Raft;
use raft_store::raft::raft_proto::{JoinRequest, PrepareRequest};
use raft_store::raft::raft_proto::raft_client::RaftClient;
use raft_store::raft::raft_proto::raft_server::{Raft as RaftService, RaftServer};

use crate::app::{App, S3Config};
use crate::services::file_transfer_service::file_transfer_proto::file_transfer_service_server::FileTransferServiceServer;
use crate::services::file_transfer_service::FileTransferServiceImpl;
use crate::services::object_storage_service::ObjectStorageServiceImpl;
use crate::services::object_storage_service::storage_proto::object_storage_service_server::ObjectStorageServiceServer;

mod services;
mod app;

/// OPTICS
#[derive(Parser, Debug)]
#[command(author = "Tongkun Zhang", version = "0.1.0", about = "Optimized Data transfer in cloud systems", long_about = None)]
#[clap(group(
ArgGroup::new("cluster")
.required(true)
.args(& ["init", "join"]),
))]
struct Args {
    /// The existing cluster address to join (e.g. 10.0.0.1:7051)
    #[arg(long = "join", value_name = "address")]
    join: Option<String>,

    /// If set, a new cluster will be created
    #[arg(long)]
    init: bool,

    /// The Hostname to advertise to other nodes
    #[arg(long, value_name = "hostname")]
    hostname: Option<String>,

    /// The address to listen on
    #[arg(short = 'a', long, default_value = "[::1]", value_name = "address")]
    listen_addr: String,

    /// The port to listen on
    #[arg(short = 'p', long, default_value = "7051", value_name = "port")]
    listen_port: u16,

    /// The working directory. If not set, a temporary directory will be created
    #[arg(short = 'd', long, value_name = "path")]
    work_dir: Option<PathBuf>,

    /// The Data directory. If not set, a temporary directory will be created
    #[arg(long, value_name = "path")]
    data_dir: Option<PathBuf>,

    /// The S3 endpoint. If not set, it will be read from the S3_ENDPOINT environment variable
    #[arg(long = "s3-endpoint", env = "S3_ENDPOINT", value_name = "endpoint")]
    s3_endpoint: String,

    /// Use path style for S3 URL. If not set, it will be read from the S3_PATH_STYLE environment variable
    #[arg(long = "s3-path-style", env = "S3_PATH_STYLE")]
    s3_path_style: Option<bool>,

    /// The S3 bucket name. If not set, it will be read from the S3_BUCKET environment variable
    #[arg(long = "s3-bucket", env = "S3_BUCKET", value_name = "bucket")]
    s3_bucket: String,

    /// The S3 secret access key. If not set, it will be read from the S3_SECRET_KEY environment variable
    #[arg(long = "s3-secret-key", env = "S3_SECRET_KEY", hide_env_values = true, value_name = "secret_key")]
    s3_secret_key: String,

    /// The S3 access key id. If not set, it will be read from the S3_ACCESS_KEY_ID environment variable
    #[arg(long = "s3-access-key-id", env = "S3_ACCESS_KEY_ID", hide_env_values = true, value_name = "access_key_id")]
    s3_access_key_id: String,
}

impl Args {
    /// Validates the provided arguments and returns a Result
    pub fn validate(&self) -> Result<(), &'static str> {
        // Either init or join should be set
        if !self.init && self.join.is_none() {
            return Err("Either 'init' or 'join' should be set.");
        }

        // Validate existing_cluster
        if let Some(ref addr) = self.join {
            if !addr.contains(':') {
                return Err("The 'join' address should be in the format 'IP:PORT'.");
            }
        }

        // Validate listen_port
        if self.listen_port == 0 {
            return Err("The 'listen_port' should be a non-zero value.");
        }
        // Add any other validation logic as needed.

        Ok(())
    }

    pub fn set_defaults(&mut self) {
        if self.work_dir.is_none() {
            // If `work_dir` is not provided, set it to a new temporary directory.
            let temp_path = tempfile::tempdir().expect("Failed to create a temporary directory");
            self.work_dir = Some(temp_path.into_path());
        }

        if self.data_dir.is_none() {
            // If `data_dir` is not provided, set it to a new temporary directory.
            let temp_path = tempfile::tempdir().expect("Failed to create a temporary directory");
            self.data_dir = Some(temp_path.into_path());
        }

        if self.hostname.is_none() {
            // If `hostname` is not provided, set it to the hostname of the current machine.
            self.hostname = Some(hostname::get().expect("Failed to get hostname").into_string().unwrap());
        }
    }
}

#[tokio::main]
async fn main() {
    let mut args = Args::parse();

    if let Err(e) = args.validate() {
        // Handle the validation error.
        eprintln!("Validation error: {}", e);
        std::process::exit(1);
    }


    args.set_defaults();
    println!("{:?}", args);

    let _ = start(args).await;
    ()
}

async fn start(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let node_id = if let Some(leader_addr) = args.join.clone() {
        let id = get_next_id_from_cluster(&leader_addr).await?;
        id
    } else {
        1
    };

    let shared_state = App {
        hostname: args.hostname.clone().unwrap(),
        addr: format!("{}:{}", args.hostname.as_deref().unwrap_or("localhost"), args.listen_port),
        store: KVStore::new(node_id, args.data_dir.unwrap(), format!("{}:{}", args.hostname.as_deref().unwrap_or("localhost"), args.listen_port), node_id == 1).await,
        s3_config: S3Config {
            endpoint: args.s3_endpoint,
            path_style: args.s3_path_style.unwrap_or(false),
            bucket: args.s3_bucket,
            secret_key: args.s3_secret_key,
            access_key_id: args.s3_access_key_id,
        },
    };
    let shared_state = Arc::new(shared_state);

    let raft_service = Raft::create(shared_state.clone().store.app.clone());
    let storage_service = ObjectStorageServiceImpl::create(shared_state.clone());
    let file_transfer_service = FileTransferServiceImpl::create(args.work_dir.unwrap());

    let listen_addr = format!("{}:{}", args.listen_addr, args.listen_port).parse()?;
    let server_handle = tokio::spawn(async move {
        Server::builder()
            .add_service(RaftServer::new(raft_service))
            .add_service(ObjectStorageServiceServer::new(storage_service))
            .add_service(FileTransferServiceServer::new(file_transfer_service))
            .serve(listen_addr)
            .await
    });

    println!("Server listening on {}", listen_addr);

    if let Some(leader_addr) = args.join {
        async_std::task::sleep(Duration::from_millis(200)).await;
        join_cluster(format!("{}:{}", args.hostname.as_deref().unwrap_or("localhost"), args.listen_port), node_id, &leader_addr).await?;
    }


    // Block on the server task.
    server_handle.await??;

    Ok(())
}


async fn get_next_id_from_cluster(leader_addr: &String) -> Result<u64, Box<dyn Error>> {
    println!("Get next id from cluster: {}", leader_addr);
    let leader_addr = format!("http://{}", leader_addr);
    let mut client = RaftClient::connect(leader_addr.clone()).await?;

    let request = tonic::Request::new(PrepareRequest { addr: "".to_string() });

    let response = client.prepare(request).await?;
    let node_id = response.into_inner().node_id;
    println!("Got node_id: {}", node_id);
    Ok(node_id)
}

async fn join_cluster(addr: String, node_id: u64, leader_addr: &String) -> Result<(), Box<dyn Error>> {
    println!("Join cluster: {} {}", addr, node_id);
    let leader_addr = format!("http://{}", leader_addr);
    let mut client = RaftClient::connect(leader_addr.clone()).await?;

    let request = tonic::Request::new(JoinRequest {
        node_id,
        addr,
    });

    let response = client.join(request).await?;
    let node_id = response.into_inner().node_id;
    println!("Got node_id: {}", node_id);
    Ok(())
}
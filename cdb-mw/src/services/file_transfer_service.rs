use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;

use futures::stream::Stream;
use tonic::{Request, Response, Status};

use crate::services::file_transfer_service::file_transfer_proto::{Chunk, DownloadRequest};
use crate::services::file_transfer_service::file_transfer_proto::file_transfer_service_server::FileTransferService;

pub mod file_transfer_proto {
    tonic::include_proto!("filetransfer");
}

pub struct FileTransferServiceImpl {
    working_dir: String,
}

impl FileTransferServiceImpl {
    pub fn create(working_dir: PathBuf) -> Self {
        Self { working_dir: working_dir.to_str().unwrap().to_string() }
    }
}

const CHUNK_SIZE: usize = 8192;  // 8KB, but you can adjust as needed

#[tonic::async_trait]
impl FileTransferService for FileTransferServiceImpl {
    type DownloadStream = std::pin::Pin<Box<dyn Stream<Item=Result<Chunk, Status>> + Send + Sync>>;

    async fn download(
        &self,
        request: Request<DownloadRequest>,
    ) -> Result<Response<Self::DownloadStream>, Status> {
        let file_name = request.into_inner().file_name;
        let file_path = format!("{}/{}", self.working_dir, file_name);  // Adjust the path as needed

        let file = match File::open(&file_path) {
            Ok(file) => file,
            Err(_) => {
                return Err(Status::not_found(format!("File {} not found", file_name)));
            }
        };

        let reader = BufReader::new(file);

        let stream = futures::stream::unfold(reader, |mut reader| {
            let mut buffer = vec![0; CHUNK_SIZE];
            Box::pin(async move {
                match reader.read_exact(&mut buffer) {
                    Ok(_) => Some((Ok(Chunk { content: buffer.into() }), reader)),
                    Err(_) => None
                }
            })
        });

        Ok(Response::new(Box::pin(stream)))
    }
}
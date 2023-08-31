use futures::stream::Stream;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use tonic::{Request, Response, Status};
use tracing::{error, info};

use crate::services::file_transfer_service::file_transfer_proto::file_transfer_service_server::FileTransferService;
use crate::services::file_transfer_service::file_transfer_proto::{Chunk, DownloadRequest};

pub mod file_transfer_proto {
    tonic::include_proto!("filetransfer");
}

pub struct FileTransferServiceImpl {
    working_dir: String,
}

impl FileTransferServiceImpl {
    pub fn create(working_dir: PathBuf) -> Self {
        Self {
            working_dir: working_dir.to_str().unwrap().to_string(),
        }
    }
}

const CHUNK_SIZE: usize = 8192; // 8KB, but you can adjust as needed

#[tonic::async_trait]
impl FileTransferService for FileTransferServiceImpl {
    type DownloadStream =
        std::pin::Pin<Box<dyn Stream<Item = Result<Chunk, Status>> + Send + Sync>>;

    async fn download(
        &self,
        request: Request<DownloadRequest>,
    ) -> Result<Response<Self::DownloadStream>, Status> {
        std::env::set_current_dir(&self.working_dir).unwrap();

        let file_name = request.into_inner().file_name;
        let file_path = format!("{}", file_name); // Adjust the path as needed

        info!("Attempting to send file {}", file_name);

        let file = match File::open(&file_path) {
            Ok(file) => file,
            Err(e) => {
                error!("File {} not found: {}", file_name, e);
                return Err(Status::not_found(format!("File {} not found", file_name)));
            }
        };

        info!("File {} found, preparing for transfer", file_name);

        let reader = BufReader::new(file);

        let stream = futures::stream::unfold(reader, |mut reader| {
            let mut buffer = vec![0; CHUNK_SIZE];
            Box::pin(async move {
                match reader.read_exact(&mut buffer) {
                    Ok(_) => Some((
                        Ok(Chunk {
                            content: buffer.into(),
                        }),
                        reader,
                    )),
                    Err(_) => None,
                }
            })
        });

        info!("File {} successfully sent", file_name);

        Ok(Response::new(Box::pin(stream)))
    }
}

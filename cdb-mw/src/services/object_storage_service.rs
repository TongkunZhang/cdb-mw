use std::pin::Pin;
use std::sync::Arc;
use std::time::SystemTime;

use futures::Stream;
use s3::{Bucket, Region};
use s3::creds::Credentials;
use tonic::{Request, Response, Status};
use tracing::{error, info, instrument};

use crate::app::App;
use crate::services::file_transfer_service::file_transfer_proto::DownloadRequest;
use crate::services::file_transfer_service::file_transfer_proto::file_transfer_service_client::FileTransferServiceClient;
use crate::services::object_storage_service::storage_proto::{
    GetObjectReferenceRequest, GetObjectS3Request, ObjectData, operation_status, OperationStatus,
    PutObjectReferenceRequest, PutObjectS3Request,
};
use crate::services::object_storage_service::storage_proto::object_storage_service_server::ObjectStorageService;

pub mod storage_proto {
    tonic::include_proto!("objectstorage");
}

pub struct ObjectStorageServiceImpl {
    app: Arc<App>,
}

impl ObjectStorageServiceImpl {
    pub fn create(app: Arc<App>) -> Self {
        Self { app }
    }
}


#[tonic::async_trait]
impl ObjectStorageService for ObjectStorageServiceImpl {
    type GetObjectS3Stream = Pin<Box<dyn Stream<Item=Result<ObjectData, Status>> + Send + Sync>>;
    type GetObjectReferenceStream = Pin<Box<dyn Stream<Item=Result<ObjectData, Status>> + Send + Sync>>;

    #[instrument(skip(self, request))]
    async fn put_object_s3(
        &self,
        request: Request<tonic::Streaming<PutObjectS3Request>>,
    ) -> Result<Response<OperationStatus>, Status> {
        let mut stream = request.into_inner();

        let mut buffer = vec![];
        // let mut bucket_name = String::new();
        let mut object_key = String::new();

        while let Some(req) = stream.message().await? {
            buffer.extend(req.chunk);
            // bucket_name = req.bucket.clone();
            object_key = req.object_key.clone();
        }

        info!(object_key = %object_key, "Putting object to S3");
        // Create S3 bucket instance
        let credentials = Credentials::new(
            Some(&self.app.s3_config.access_key_id),
            Some(&self.app.s3_config.secret_key),
            None,
            None,
            None,
        )
            .unwrap(); // Handle this better
        let region = Region::Custom {
            region: "".into(),
            endpoint: self.app.s3_config.endpoint.clone(),
        };
        let mut bucket = Bucket::new(&self.app.s3_config.bucket, region, credentials).unwrap();
        if self.app.s3_config.path_style {
            bucket = bucket.with_path_style();
        }
        // Put object to S3
        let response_data = bucket
            .put_object(&object_key, &buffer)
            .await
            .map_err(|e| {
                error!(reason = %e, "Failed to put object");
                Status::internal(format!("Failed to put object: {}", e))
            })?;

        if response_data.status_code() != 200 {
            return Err(Status::internal("Failed to put object to S3"));
        }

        Ok(Response::new(OperationStatus {
            status: operation_status::Status::Success as i32,
            message: "".to_string(),
        }))
    }

    #[instrument(skip(self, request))]
    async fn get_object_s3(
        &self,
        request: Request<GetObjectS3Request>,
    ) -> Result<Response<Self::GetObjectS3Stream>, Status> {
        let request = request.into_inner();

        info!(object_key = %request.object_key, "Getting object from S3");
        // Create S3 bucket instance
        let credentials = Credentials::new(
            Some(&self.app.s3_config.access_key_id),
            Some(&self.app.s3_config.secret_key),
            None,
            None,
            None,
        )
            .unwrap(); // Handle this better
        let region = Region::Custom {
            region: "".into(),
            endpoint: self.app.s3_config.endpoint.clone(),
        };
        let mut bucket = Bucket::new(&self.app.s3_config.bucket, region, credentials).unwrap();
        if self.app.s3_config.path_style {
            bucket = bucket.with_path_style();
        }
        // Get object from S3
        let response_data = bucket.get_object(&request.object_key).await.map_err(|e| {
            error!(reason = %e, "Failed to get object");
            Status::internal(format!("Failed to get object: {}", e))
        })?;

        if response_data.status_code() != 200 {
            return Err(Status::internal("Failed to get object from S3"));
        }

        let object_data = response_data.as_slice().to_vec();

        let (tx, rx) = tokio::sync::mpsc::channel(100);

        // Spawning a new task to break the object into smaller chunks
        tokio::spawn(async move {
            const CHUNK_SIZE: usize = 1024;
            for chunk in object_data.chunks(CHUNK_SIZE) {
                let message = ObjectData {
                    chunk: chunk.to_vec(),
                };
                tx.send(Ok(message)).await.unwrap();
            }
        });

        Ok(Response::new(Box::pin(
            tokio_stream::wrappers::ReceiverStream::new(rx),
        )))
    }

    #[instrument(skip(self, request))]
    async fn put_object_reference(
        &self,
        request: Request<PutObjectReferenceRequest>,
    ) -> Result<Response<OperationStatus>, Status> {
        let request = request.into_inner();

        let key = serialize_obj_ref_key(&request.object_key);

        // Check if the path exists
        let path = request.path.clone();
        let path_exists = std::path::Path::new(&path).exists();
        if !path_exists {
            return Err(Status::internal(format!(
                "Path {} does not exist",
                request.path
            )));
        }

        let value = serialize_obj_ref_value(&request.path, 0, &self.app.addr);
        info!(
            object_key = %request.object_key,
            path = %request.path,
            "Putting object reference"
        );

        self.app.store.write(&key, &value).await;

        Ok(Response::new(OperationStatus {
            status: operation_status::Status::Success as i32,
            message: "".to_string(),
        }))
    }

    #[instrument(skip(self, request))]
    async fn get_object_reference(
        &self,
        request: Request<GetObjectReferenceRequest>,
    ) -> Result<Response<Self::GetObjectReferenceStream>, Status> {
        let request = request.into_inner();

        let key = serialize_obj_ref_key(&request.object_key);
        info!(object_key = %request.object_key, "Getting object reference for key");

        let value = self.app.store.read(&key).await;
        match value {
            Ok(ref_value) => {
                let (path, _, addr) = deserialize_obj_ref_value(&ref_value);

                info!(object_key = %request.object_key, path = %path, "Getting object reference for key");

                // Use addr to connect to the FileTransferService and get the file

                let object_data = download_from_file_transfer_service(&addr, &path).await?;
                let (tx, rx) = tokio::sync::mpsc::channel(100);
                // Spawning a new task to break the object into smaller chunks
                tokio::spawn(async move {
                    const CHUNK_SIZE: usize = 1024;
                    for chunk in object_data.chunks(CHUNK_SIZE) {
                        let message = ObjectData {
                            chunk: chunk.to_vec(),
                        };
                        if let Err(e) = tx.send(Ok(message)).await {
                            error!(reason = %e, "Failed to send chunk");
                            break;
                        }
                    }
                });

                Ok(Response::new(Box::pin(
                    tokio_stream::wrappers::ReceiverStream::new(rx),
                )))
            }
            Err(e) => Err(Status::internal(format!(
                "Failed to get object reference: {}",
                e
            ))),
        }
    }
}

#[instrument(skip(addr, path))]
async fn download_from_file_transfer_service(addr: &str, path: &str) -> Result<Vec<u8>, Status> {
    info!(addr = %addr, path = %path, "Downloading file from FileTransferService");
    // Assume addr provides the address to connect to FileTransferService
    let mut client = FileTransferServiceClient::connect(format!("http://{}", addr))
        .await
        .map_err(|e| Status::internal(format!("Failed to connect: {}", e)))?;

    let request = tonic::Request::new(DownloadRequest {
        file_name: path.to_string(),
    });

    let mut stream = client
        .download(request)
        .await
        .map_err(|e| {
            error!(reason = %e, "Download failed");
            Status::internal(format!("Download failed: {}", e))
        })?
        .into_inner();
    let mut data = Vec::new();

    while let Some(chunk) = stream.message().await.map_err(|e| {
        error!(reason = %e, "Stream error");
        Status::internal(format!("Stream error: {}", e))
    })? {
        data.extend(chunk.content);
    }

    info!(addr = %addr, path = %path, "File downloaded from FileTransferService");

    Ok(data)
}

fn serialize_obj_ref_key(object_key: &str) -> String {
    format!("refs/{}", object_key)
}

fn serialize_obj_ref_value(path: &str, mut created_time: u64, addr: &str) -> String {
    if created_time == 0 {
        created_time = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
    format!("{}|{}|{}", path, created_time, addr)
}

fn deserialize_obj_ref_value(value: &str) -> (String, u64, String) {
    let mut parts = value.split('|');
    let path = parts.next().unwrap().to_string();
    let created_time = parts.next().unwrap().parse::<u64>().unwrap();
    let addr = parts.next().unwrap().to_string();
    (path, created_time, addr)
}

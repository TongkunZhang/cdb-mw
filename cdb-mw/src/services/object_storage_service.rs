use std::sync::Arc;
use std::time::SystemTime;

use s3::{Bucket, Region};
use s3::creds::Credentials;
use tonic::{Request, Response, Status};

use crate::app::App;
use crate::services::file_transfer_service::file_transfer_proto::DownloadRequest;
use crate::services::file_transfer_service::file_transfer_proto::file_transfer_service_client::FileTransferServiceClient;
use crate::services::object_storage_service::storage_proto::{GetObjectReferenceRequest, GetObjectS3Request, ObjectData, operation_status, OperationStatus, PutObjectReferenceRequest, PutObjectS3Request};
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
    async fn put_object_s3(
        &self,
        request: Request<PutObjectS3Request>,
    ) -> Result<Response<OperationStatus>, Status> {
        let request = request.into_inner();

// Create S3 bucket instance
        let credentials = Credentials::new(
            Some(&self.app.s3_config.access_key_id),
            Some(&self.app.s3_config.secret_key),
            None, None, None,
        ).unwrap(); // Handle this better
        let region = Region::Custom {
            region: "".into(),
            endpoint: self.app.s3_config.endpoint.clone(),
        };
        let mut bucket = Bucket::new(
            &self.app.s3_config.bucket,
            region,
            credentials,
        ).unwrap();
        if self.app.s3_config.path_style {
            bucket = bucket.with_path_style();
        }
        // Put object to S3
        let response_data = bucket.put_object(&request.object_key, &request.data).await
            .map_err(|e| Status::internal(format!("Failed to put object: {}", e)))?;

        if response_data.status_code() != 200 {
            return Err(Status::internal("Failed to put object to S3"));
        }

        Ok(Response::new(OperationStatus {
            status: operation_status::Status::Success as i32,
            message: "".to_string(),
        }))
    }

    async fn get_object_s3(
        &self,
        request: Request<GetObjectS3Request>,
    ) -> Result<Response<ObjectData>, Status> {
        let request = request.into_inner();

        // Create S3 bucket instance
        let credentials = Credentials::new(
            Some(&self.app.s3_config.access_key_id),
            Some(&self.app.s3_config.secret_key),
            None, None, None,
        ).unwrap(); // Handle this better
        let region = Region::Custom {
            region: "".into(),
            endpoint: self.app.s3_config.endpoint.clone(),
        };
        let mut bucket = Bucket::new(
            &self.app.s3_config.bucket,
            region,
            credentials,
        ).unwrap();
        if self.app.s3_config.path_style {
            bucket = bucket.with_path_style();
        }
        // Get object from S3
        let response_data = bucket.get_object(&request.object_key).await
            .map_err(|e| Status::internal(format!("Failed to get object: {}", e)))?;

        if response_data.status_code() != 200 {
            return Err(Status::internal("Failed to get object from S3"));
        }

        Ok(Response::new(ObjectData {
            data: response_data.as_slice().to_vec(),
        }))
    }

    async fn put_object_reference(&self, request: Request<PutObjectReferenceRequest>) -> Result<Response<OperationStatus>, Status> {
        let request = request.into_inner();

        let key = serialize_obj_ref_key(&request.object_key);
        let value = serialize_obj_ref_value(&request.path, 0, &self.app.addr);
        self.app.store.write(&key, &value).await;

        Ok(Response::new(OperationStatus {
            status: operation_status::Status::Success as i32,
            message: "".to_string(),
        }))
    }

    async fn get_object_reference(&self, request: Request<GetObjectReferenceRequest>) -> Result<Response<ObjectData>, Status> {
        let request = request.into_inner();

        let key = serialize_obj_ref_key(&request.object_key);
        let value = self.app.store.read(&key).await;
        match value {
            Ok(ref_value) => {
                let (path, _, addr) = deserialize_obj_ref_value(&ref_value);

                // Use addr to connect to the FileTransferService and get the file
                let file_data = download_from_file_transfer_service(&addr, &path).await?;

                // Return the data from the downloaded file
                Ok(Response::new(ObjectData {
                    data: file_data,
                }))
            }
            Err(e) => {
                Err(Status::internal(format!("Failed to get object reference: {}", e)))
            }
        }
    }
}

async fn download_from_file_transfer_service(
    addr: &str,
    path: &str,
) -> Result<Vec<u8>, Status> {
    // Assume addr provides the address to connect to FileTransferService
    let mut client = FileTransferServiceClient::connect(addr.to_string()).await.map_err(|e| Status::internal(format!("Failed to connect: {}", e)))?;

    let request = tonic::Request::new(DownloadRequest {
        file_name: path.to_string(),
    });

    let mut stream = client.download(request).await.map_err(|e| Status::internal(format!("Download failed: {}", e)))?.into_inner();
    let mut data = Vec::new();

    while let Some(chunk) = stream.message().await.map_err(|e| Status::internal(format!("Stream error: {}", e)))? {
        data.extend(chunk.content);
    }

    Ok(data)
}

fn serialize_obj_ref_key(object_key: &str) -> String {
    format!("refs/{}", object_key)
}

fn serialize_obj_ref_value(path: &str, mut created_time: u64, addr: &str) -> String {
    if created_time == 0 {
        created_time = SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
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

fn deserialize_obj_ref_key(key: &str) -> (String) {
    let mut parts = key.split('/');
    let object_key = parts.next().unwrap().to_string();
    (object_key)
}



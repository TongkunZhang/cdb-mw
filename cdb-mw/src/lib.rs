use std::error::Error;
use std::pin::Pin;

use futures::Stream;
use tokio_stream::StreamExt;
use tonic::Request;

use crate::storage_proto::{GetObjectReferenceRequest, GetObjectS3Request, PutObjectReferenceRequest, PutObjectS3Request};
use crate::storage_proto::object_storage_service_client::ObjectStorageServiceClient;

pub mod storage_proto {
    tonic::include_proto!("objectstorage");
}

pub struct OpticsHandler {
    client: ObjectStorageServiceClient<tonic::transport::Channel>,
}

impl OpticsHandler {
    pub async fn new(endpoint: String) -> Result<Self, Box<dyn Error>> {
        let client = ObjectStorageServiceClient::connect(endpoint).await?;
        Ok(Self { client })
    }

    pub async fn put_s3(&mut self, object_key: String, data: Vec<u8>) -> Result<(), Box<dyn Error>> {
        let chunk_size = 1024 * 1024 * 2; // 2MB

        let object_key = object_key.trim().to_string();

        let request_stream: Pin<Box<dyn Stream<Item=PutObjectS3Request> + Send + Sync>> = Box::pin(async_stream::stream! {
        for (i, chunk) in data.chunks(chunk_size).enumerate() {
            println!("Sending chunk {}", i);
            let request = PutObjectS3Request {
                object_key: object_key.clone(),
                chunk: chunk.to_vec().into(),
            };
            yield request;
        }
    });

        let response = self.client.put_object_s3(request_stream).await?;
        println!("Response: {:?}", response);

        Ok(())
    }


    pub async fn get_s3(&mut self, object_key: String) -> Result<Vec<u8>, Box<dyn Error>> {
        let request = GetObjectS3Request {
            object_key: object_key.trim().to_string(),
        };
        let mut stream = self.client.get_object_s3(Request::new(request)).await?.into_inner();

        let mut data: Vec<u8> = Vec::new();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(chunk_data) => {
                    data.extend_from_slice(&chunk_data.chunk);
                }
                Err(e) => {
                    return Err(Box::new(e));
                }
            }
        }

        Ok(data)
    }

    pub async fn put_obj_ref(&mut self, object_key: String, ref_value: String) -> Result<(), Box<dyn Error>> {
        let request = PutObjectReferenceRequest {
            object_key,
            path: ref_value,
        };

        let response = self.client.put_object_reference(Request::new(request)).await?;
        println!("Response: {:?}", response);

        Ok(())
    }

    pub async fn get_obj_ref(&mut self, object_key: String) -> Result<Vec<u8>, Box<dyn Error>> {
        let request = GetObjectReferenceRequest {
            object_key: object_key.trim().to_string(),
        };

        let mut stream = self.client.get_object_reference(Request::new(request)).await?.into_inner();

        let mut data: Vec<u8> = Vec::new();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(chunk_data) => {
                    data.extend_from_slice(&chunk_data.chunk);
                }
                Err(e) => {
                    return Err(Box::new(e));
                }
            }
        }

        Ok(data)
    }
}

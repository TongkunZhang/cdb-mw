use std::env;
use std::fs;
use std::io::{self, Write};

use crate::services::object_storage_service::storage_proto::{GetObjectS3Request, PutObjectS3Request};
use crate::services::object_storage_service::storage_proto::object_storage_service_client::ObjectStorageServiceClient;

mod services;
mod app;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <server_addr> <port>", args[0]);
        std::process::exit(1);
    }

    let server_addr = &args[1];
    let port = &args[2];

    let endpoint = format!("http://{}:{}", server_addr, port);
    let mut client = ObjectStorageServiceClient::connect(endpoint).await?;

    loop {
        let mut input = String::new();
        print!("Choose an action (put_s3/get_s3/put_ref/get_ref/quit): ");
        io::stdout().flush()?;
        io::stdin().read_line(&mut input)?;

        match input.trim() {
            "put_s3" => {
                print!("Enter object key: ");
                io::stdout().flush()?;
                let mut object_key = String::new();
                io::stdin().read_line(&mut object_key)?;

                print!("Enter file path to upload: ");
                io::stdout().flush()?;
                let mut file_path = String::new();
                io::stdin().read_line(&mut file_path)?;

                let data = fs::read(file_path.trim())?;

                let request = PutObjectS3Request {
                    bucket: "".to_string(),  // This should probably be set dynamically
                    object_key: object_key.trim().to_string(),
                    data: data.into(),
                };

                let response = client.put_object_s3(tonic::Request::new(request)).await?;
                println!("Response: {:?}", response);
            }
            "get_s3" => {
                print!("Enter object key to retrieve: ");
                io::stdout().flush()?;
                let mut object_key = String::new();
                io::stdin().read_line(&mut object_key)?;

                print!("Enter file path to save retrieved object: ");
                io::stdout().flush()?;
                let mut file_path = String::new();
                io::stdin().read_line(&mut file_path)?;

                let request = GetObjectS3Request {
                    bucket: "".to_string(),  // This should probably be set dynamically
                    object_key: object_key.trim().to_string(),
                };

                let response = client.get_object_s3(tonic::Request::new(request)).await?;
                fs::write(file_path.trim(), response.into_inner().data)?;
                println!("Object saved successfully!");
            }
            "put_ref" => {
                print!("Enter object key: ");
                io::stdout().flush()?;
                let mut object_key = String::new();
                io::stdin().read_line(&mut object_key)?;

                print!("Enter reference value (e.g., URL or path): ");
                io::stdout().flush()?;
                let mut ref_value = String::new();
                io::stdin().read_line(&mut ref_value)?;

                print!("Putting object reference for key {} with path {}", object_key.trim(), ref_value.trim());
                let request = services::object_storage_service::storage_proto::PutObjectReferenceRequest {
                    object_key: object_key.trim().to_string(),
                    path: ref_value.trim().to_string(),
                };

                let response = client.put_object_reference(tonic::Request::new(request)).await?;
                println!("Response: {:?}", response);
            }

            "get_ref" => {
                print!("Enter hostname of node that has the reference: ");
                io::stdout().flush()?;
                let mut hostname = String::new();
                io::stdin().read_line(&mut hostname)?;
                print!("Enter object key for which you want the reference: ");
                io::stdout().flush()?;
                let mut object_key = String::new();
                io::stdin().read_line(&mut object_key)?;

                println!("Getting object reference for key {} from node {}", object_key.trim(), hostname.trim());

                let request = services::object_storage_service::storage_proto::GetObjectReferenceRequest {
                    hostname,
                    object_key: object_key.trim().to_string(),
                };

                let response = client.get_object_reference(tonic::Request::new(request)).await?;
                let data = response.into_inner().data;

                let mut file_path = String::new();
                print!("Enter file path to save retrieved object: ");
                io::stdout().flush()?;
                io::stdin().read_line(&mut file_path)?;
                fs::write(file_path.trim(), data)?;
                println!("Object saved successfully!");

            },

            "quit" => break,
            _ => println!("Unknown command!"),
        }
    }

    Ok(())
}

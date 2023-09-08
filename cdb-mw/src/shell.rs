use std::env;
use std::fs;
use std::io::{self, Write};

// use tokio::task::JoinHandle;

use optics::OpticsHandler;

fn prompt_for_input(prompt: &str) -> Result<String, std::io::Error> {
    let mut user_input = String::new();
    print!("{}", prompt);
    io::stdout().flush()?;
    io::stdin().read_line(&mut user_input)?;
    Ok(String::from(user_input.trim()))
}

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
    let mut handler = OpticsHandler::new(endpoint).await?;

    // let mut background_tasks: Vec<JoinHandle<()>> = Vec::new();

    loop {
        let action = prompt_for_input("Choose an action (put_s3[_async]/get_s3[_async]/put_ref[_async]/get_ref[_async]/quit): ")?;

        match action.trim() {
            "put_s3" => {
                let object_key = prompt_for_input("Enter object key: ")?.trim().to_string();
                let file_path = prompt_for_input("Enter file path to upload: ")?;

                let data = fs::read(file_path.trim()).unwrap_or_else(|_| vec![]); // Handle the error as you see fit
                handler.put_s3(object_key, data).await?;
            }
            "get_s3" => {
                let object_key = prompt_for_input("Enter object key: ")?;
                let file_path = prompt_for_input("Enter file path to save retrieved object: ")?;

                let mut file = fs::File::create(file_path.trim())?;

                let data = handler.get_s3(object_key).await?;
                file.write_all(&data)?;

                println!("Object saved successfully!");
            }
            "put_ref" => {
                let object_key = prompt_for_input("Enter object key: ")?;
                let ref_value = prompt_for_input("Enter reference value (e.g., URL or path): ")?;

                print!(
                    "Putting object reference for key {} with path {}",
                    object_key.trim(),
                    ref_value.trim()
                );
                handler.put_obj_ref(object_key.trim().to_string(), ref_value.trim().to_string()).await?;
            }
            "get_ref" => {
                let object_key = prompt_for_input("Enter object key: ")?;
                let file_path = prompt_for_input("Enter file path to save retrieved object: ")?;

                let mut file = fs::File::create(file_path.trim())?;

                let data = handler.get_obj_ref(object_key).await?;
                file.write_all(&data)?;

                println!("Object saved successfully!");
            }
            // "put_s3_async" => {
            //     let object_key = prompt_for_input("Enter object key: ")?.trim().to_string();
            //     let file_path = prompt_for_input("Enter file path to upload: ")?;
            //
            //     let data = fs::read(file_path.trim()).unwrap_or_else(|_| vec![]); // Handle the error as you see fit
            //     let task = tokio::spawn(async move {
            //         handler.put_s3(object_key, data).await.unwrap();
            //         println!("put_s3 operation for {} completed", object_key);
            //     });
            //
            //     background_tasks.push(task);
            // }
            "quit" => break,
            _ => println!("Unknown command!"),
        }
    }

    Ok(())
}

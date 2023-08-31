#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;
    use std::time::Duration;

    use tokio::runtime::Runtime;
    use tonic::transport::Server;

    use raft_store::raft::raft_proto::raft_server::RaftServer;
    use raft_store::raft::Raft;
    use raft_store::{KVStore, Node};

    // use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

    #[test]
    fn test_kvstore_integration() {
        // tracing_subscriber::registry()
        //     .with(fmt::layer())
        //     .init();
        let _ = fs::remove_dir_all("node1_data");
        let _ = fs::remove_dir_all("node2_data");
        let _ = fs::remove_dir_all("node3_data");

        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Initialize 3 nodes
            println!("=== init nodes");
            let node1 =
                Arc::new(KVStore::new(1, "node1_data", "0.0.0.0:50051".to_string(), true).await);
            let node2 =
                Arc::new(KVStore::new(2, "node2_data", "0.0.0.0:50052".to_string(), false).await);
            let node3 =
                Arc::new(KVStore::new(3, "node3_data", "0.0.0.0:50053".to_string(), false).await);

            // Start 3 servers
            println!("=== start servers");
            let n1_clone = node1.clone();
            tokio::spawn(async move {
                Server::builder()
                    .add_service(RaftServer::new(Raft::create(n1_clone.app.clone())))
                    .serve("127.0.0.1:50051".parse().unwrap())
                    .await
                    .unwrap();
            });

            let n2_clone = node2.clone();
            tokio::spawn(async move {
                Server::builder()
                    .add_service(RaftServer::new(Raft::create(n2_clone.app.clone())))
                    .serve("127.0.0.1:50052".parse().unwrap())
                    .await
                    .unwrap();
            });

            let n3_clone = node3.clone();
            tokio::spawn(async move {
                Server::builder()
                    .add_service(RaftServer::new(Raft::create(n3_clone.app.clone())))
                    .serve("127.0.0.1:50053".parse().unwrap())
                    .await
                    .unwrap();
            });
            // Pause a bit to ensure servers start (not ideal, but simple for demonstration)
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            match node1
                .app
                .raft
                .add_learner(
                    2,
                    Node {
                        grpc_addr: "127.0.0.1:50052".to_string(),
                    },
                    true,
                )
                .await
            {
                Ok(_) => println!("add learner success"),
                Err(e) => println!("add learner error: {:?}", e),
            }

            match node1
                .app
                .raft
                .add_learner(
                    3,
                    Node {
                        grpc_addr: "127.0.0.1:50053".to_string(),
                    },
                    true,
                )
                .await
            {
                Ok(_) => println!("add learner success"),
                Err(e) => println!("add learner error: {:?}", e),
            }

            // Change membership
            println!("=== change membership");
            let members = node1
                .app
                .raft
                .change_membership([1u64, 2u64, 3u64], false)
                .await
                .unwrap();
            let members = node1
                .app
                .raft
                .change_membership([1u64, 2u64, 3u64], false)
                .await
                .unwrap();
            println!("members: {:?}", members);

            // let metrics = node1.app.raft.metrics().borrow().clone();
            // dbg!(metrics);
            // let metrics = node2.app.raft.metrics().borrow().clone();
            // dbg!(metrics);

            println!("=== write `foo=bar`");

            node1.write("foo", "bar").await;

            // --- Wait for a while to let the replication get done.

            async_std::task::sleep(Duration::from_millis(200)).await;

            // --- Read it on every node.

            println!("=== read `foo` on node 1");
            let value = node1.read("foo").await.unwrap();
            assert_eq!(value, "bar");

            println!("=== read `foo` on node 2");
            let value = node2.read("foo").await.unwrap();
            assert_eq!(value, "bar");

            println!("=== read `foo` on node 3");
            let value = node3.read("foo").await.unwrap();
            assert_eq!(value, "bar");

            // --- Write a new value on node 1.
            println!("=== write `foo=baz` on node 1");
            node1.write("foo", "baz").await;

            // --- Wait for a while to let the replication get done.
            async_std::task::sleep(Duration::from_millis(200)).await;

            // --- Read it on every node.
            println!("=== read `foo` on node 1");
            let value = node1.read("foo").await.unwrap();
            assert_eq!(value, "baz");

            println!("=== read `foo` on node 2");
            let value = node2.read("foo").await.unwrap();
            assert_eq!(value, "baz");

            println!("=== read `foo` on node 3");
            let value = node3.read("foo").await.unwrap();
            assert_eq!(value, "baz");

            // --- Write a new value on node 2.
            println!("=== write `foo=qux` on node 2");
            node2.write("foo", "qux").await;

            // --- Wait for a while to let the replication get done.
            async_std::task::sleep(Duration::from_millis(200)).await;

            // --- Read it on every node.
            println!("=== read `foo` on node 1");
            let value = node1.read("foo").await.unwrap();
            assert_eq!(value, "qux");

            println!("=== read `foo` on node 2");
            let value = node2.read("foo").await.unwrap();
            assert_eq!(value, "qux");
            //
            println!("=== read `foo` on node 3");
            let value = node3.read("foo").await.unwrap();
            assert_eq!(value, "qux");
        });

        // Clean up data directories
        fs::remove_dir_all("node1_data").unwrap();
        fs::remove_dir_all("node2_data").unwrap();
        fs::remove_dir_all("node3_data").unwrap();
    }
}

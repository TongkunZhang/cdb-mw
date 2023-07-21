// use std::backtrace::Backtrace;
// use std::collections::BTreeMap;
// use std::panic::PanicInfo;
// use std::thread;
// use std::time::Duration;
//
// use async_std::task::block_on;
// use raft_store::raft::raft_proto::raft_server::RaftServer;
// use raft_store::raft::Raft;
// use tonic::transport::Server;
//
// pub fn log_panic(panic: &PanicInfo) {
//     let backtrace = { format!("{:?}", Backtrace::force_capture()) };
//
//     eprintln!("{}", panic);
//
//     if let Some(location) = panic.location() {
//         eprintln!(
//             "{}:{}:{}",
//             location.file(),
//             location.line(),
//             location.column()
//         );
//     } else {
//         eprintln!("no location information available.");
//     }
//
//     eprintln!("{}", backtrace);
// }
//
// /// Setup a cluster of 3 nodes.
// /// Write to it and read from it.
// #[async_std::test(flavor = "multi_thread", worker_threads = 8)]
// async fn test_cluster() -> Result<(), Box<dyn std::error::Error>> {
//     // --- The client itself does not store addresses for all nodes, but just node id.
//     //     Thus we need a supporting component to provide mapping from node id to node address.
//     //     This is only used by the client. A raft node in this example stores node addresses in its
//     // store.
//
//     std::panic::set_hook(Box::new(|panic| {
//         log_panic(panic);
//     }));
//
//     fn get_rpc_addr(node_id: u32) -> String {
//         match node_id {
//             1 => "127.0.0.1:22001".to_string(),
//             2 => "127.0.0.1:22002".to_string(),
//             3 => "127.0.0.1:22003".to_string(),
//             _ => panic!("node not found"),
//         }
//     }
//
//     // --- Start 3 raft node in 3 threads.
//     let d1 = tempfile::TempDir::new()?;
//     let d2 = tempfile::TempDir::new()?;
//     let d3 = tempfile::TempDir::new()?;
//
//     let _h1 = thread::spawn(async move {
//         let mut server = Server::builder();
//         let n1 = raft_store::KVStore::new(1, d1.path(), get_rpc_addr(1)).await;
//         let raft_impl = Raft::create(n1.app.clone());
//         server
//             .add_service(RaftServer::new(raft_impl))
//             .serve(get_rpc_addr(1).parse().unwrap());
//     });
//
//     // Wait for server to start up.
//     async_std::task::sleep(Duration::from_millis(500)).await;
//
//     // --- Try to write some application data through the leader.
//
//     println!("=== write `foo=bar`");
//
//     // --- Wait for a while to let the replication get done.
//
//     async_std::task::sleep(Duration::from_millis(200)).await;
//
//     // --- Read it on every node.
//
//     println!("=== read `foo` on node 1");
//
//     println!("=== read `foo` on node 2");
//
//     println!("=== read `foo` on node 3");
//
//     // --- A write to non-leader will be automatically forwarded to a known leader
//
//     println!("=== read `foo` on node 2");
//
//     async_std::task::sleep(Duration::from_millis(200)).await;
//
//     // --- Read it on every node.
//
//     println!("=== read `foo` on node 1");
//
//     println!("=== read `foo` on node 2");
//
//     println!("=== read `foo` on node 3");
//
//     println!("=== consistent_read `foo` on node 1");
//
//     println!("=== consistent_read `foo` on node 2 MUST return CheckIsLeaderError");
//
//     Ok(())
// }

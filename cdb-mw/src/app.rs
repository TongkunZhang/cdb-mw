use raft_store::KVStore;

#[allow(dead_code)]
pub struct S3Config {
    pub endpoint: String,
    pub path_style: bool,
    pub bucket: String,
    pub secret_key: String,
    pub access_key_id: String,
}

#[allow(dead_code)]
pub struct App {
    pub hostname: String,
    pub addr: String,
    pub store: KVStore,
    pub s3_config: S3Config,
}

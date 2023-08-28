use raft_store::KVStore;

pub struct S3Config {
    pub endpoint: String,
    pub path_style: bool,
    pub bucket: String,
    pub secret_key: String,
    pub access_key_id: String,
}

pub struct App {
    pub hostname: String,
    pub addr: String,
    pub store: KVStore,
    pub s3_config: S3Config,
}
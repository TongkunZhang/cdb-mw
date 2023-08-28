mod app;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        use s3::Bucket;
        use s3::creds::Credentials;
        use s3::Region;
        use tokio::runtime::Runtime;
        let bucket = Bucket::new(
            "bucket-1",
            Region::Custom {
                region: "".to_owned(),
                endpoint: "https://tenants-1.s3.tongkun.io:30443".to_owned(),
            },
            Credentials::new(
                Option::from("T1XcQ0m8taHUJIq0"),
                Option::from("yqYAZvEizgknXiXHbGe5KwpAyiELYhXO"),
                None,
                None,
                None,
            )
                .unwrap(),
        )
            .unwrap()
            .with_path_style();

        let s3_path = "test.file";
        let test = b"I'm going to S3!";

        Runtime::new().unwrap().block_on(async {
            let response_data = bucket
                .put_object(s3_path, test)
                .await
                .expect("TODO: panic message");
            assert_eq!(response_data.status_code(), 200);
        });

        Runtime::new().unwrap().block_on(async {
            let mut response_data = bucket
                .get_object(s3_path)
                .await
                .expect("TODO: panic message");
            assert_eq!(test, response_data.as_slice());
            assert_eq!(response_data.status_code(), 200);
        });
    }
}
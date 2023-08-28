# cdb-mw

## Example
```bash
S3_ENDPOINT=https://tenants-1.s3.tongkun.io:30443 S3_PATH_STYLE=true S3_BUCKET=bucket-1 S3_SECRET_KEY=yqYAZvEizgknXiXHbGe5KwpAyiELYhXO S3_ACCESS_KEY_ID=T1XcQ0m8taHUJIq0 cargo run --bin optics -- --join 127.0.0.1:7051 -p 8051 --hostname=127.0.0.1 -a 127.0.0.1
S3_ENDPOINT=https://tenants-1.s3.tongkun.io:30443 S3_PATH_STYLE=true S3_BUCKET=bucket-1 S3_SECRET_KEY=yqYAZvEizgknXiXHbGe5KwpAyiELYhXO S3_ACCESS_KEY_ID=T1XcQ0m8taHUJIq0 cargo run --bin optics -- --init -a 127.0.0.1 --hostname=127.0.0.1
S3_ENDPOINT=https://tenants-1.s3.tongkun.io:30443 S3_PATH_STYLE=true S3_BUCKET=bucket-1 S3_SECRET_KEY=yqYAZvEizgknXiXHbGe5KwpAyiELYhXO S3_ACCESS_KEY_ID=T1XcQ0m8taHUJIq0 cargo run --bin optics -- --join 127.0.0.1:7051 -p 9051 --hostname=127.0.0.1 -a 127.0.0.1
```
use anyhow::Result;
use aws_config::{BehaviorVersion, SdkConfig, retry::RetryConfig};
use aws_lambda_events::event::s3::S3Event;
use aws_sdk_s3 as s3;
use aws_sdk_sqs as sqs;
use serde::Deserialize;

#[derive(Deserialize)]
struct Event {
    bucket: String,
    key: String,
}

impl Event {
    // POC only handles 1 record per event
    fn from_s3_event(event: &S3Event) -> Event {
        event
            .records
            .first()
            .map(|record| Event {
                bucket: record
                    .s3
                    .bucket
                    .name
                    .as_ref()
                    .expect("S3 Event must contain bucket name")
                    .clone(),
                key: record
                    .s3
                    .object
                    .key
                    .as_ref()
                    .expect("S3 Event must contain S3 object key")
                    .clone(),
            })
            .expect("S3 Event must contain at least one record")
    }
}

pub async fn get_aws_config() -> SdkConfig {
    let version = BehaviorVersion::v2025_01_17();
    let retry_config = RetryConfig::adaptive().with_max_attempts(3);

    aws_config::defaults(version)
        .retry_config(retry_config)
        .load()
        .await
}

fn get_queue_url() -> Result<String> {
    Ok(std::env::var("SQS_QUEUE_URL")?)
}

async fn handle(request: lambda_runtime::LambdaEvent<S3Event>) -> Result<()> {
    let aws_config = get_aws_config().await;
    let s3_client = s3::Client::new(&aws_config);
    let sqs_client = sqs::Client::new(&aws_config);

    let event = Event::from_s3_event(&request.payload);
    let stream = producer::get_object(&s3_client, &event.bucket, &event.key).await?;

    let mut process = producer::Process::new(sqs_client, get_queue_url()?);
    process.run(stream).await;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    lambda_runtime::tracing::init_default_subscriber();

    let service_fn = lambda_runtime::service_fn(handle);
    lambda_runtime::run(service_fn).await
}

use aws_config::{BehaviorVersion, SdkConfig, retry::RetryConfig};
use aws_lambda_events::event::sqs::{SqsBatchResponse, SqsEvent};
use aws_sdk_dynamodb as dynamodb;
use lambda_runtime::tracing;

pub async fn get_aws_config() -> SdkConfig {
    let version = BehaviorVersion::v2025_01_17();
    let retry_config = RetryConfig::disabled();

    aws_config::defaults(version)
        .retry_config(retry_config)
        .load()
        .await
}

fn get_dynamodb_table() -> anyhow::Result<String> {
    Ok(std::env::var("DYNAMODB_TABLE")?)
}

async fn handle(
    request: lambda_runtime::LambdaEvent<SqsEvent>,
) -> Result<SqsBatchResponse, lambda_runtime::Error> {
    let aws_config = get_aws_config().await;
    let ddb_client = dynamodb::Client::new(&aws_config);

    let messages = &request.payload.records;
    tracing::info!("Number of messages received: {}", messages.len());

    let records = consumer::process_messages(messages);
    tracing::info!("Number of messages processed to records: {}", records.len());

    let mut process = consumer::Process::new(ddb_client, get_dynamodb_table()?);
    let batch_item_failures = process.run(records).await?;
    tracing::info!("Number of failures: {}", batch_item_failures.len());

    Ok(SqsBatchResponse {
        batch_item_failures,
    })
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    lambda_runtime::tracing::init_default_subscriber();

    let service_fn = lambda_runtime::service_fn(handle);
    lambda_runtime::run(service_fn).await
}

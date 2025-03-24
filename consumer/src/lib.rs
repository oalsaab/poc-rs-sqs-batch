use anyhow::Result;
use aws_lambda_events::sqs::{BatchItemFailure, SqsMessage};
use aws_sdk_dynamodb as dynamodb;
use aws_sdk_dynamodb::operation::batch_write_item::{BatchWriteItemError, BatchWriteItemOutput};
use aws_sdk_dynamodb::types::{AttributeValue, PutRequest, WriteRequest};
use itertools::Itertools;
use lambda_runtime::tracing;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinSet;

const BATCH_SIZE: usize = 25;

#[derive(Deserialize, Clone)]
struct Item {
    id: String,
    colour: String,
    price: i64,
}

impl Item {
    fn item(&self) -> HashMap<String, AttributeValue> {
        HashMap::from([
            ("id".into(), AttributeValue::S(self.id.clone())),
            ("colour".into(), AttributeValue::S(self.colour.clone())),
            ("price".into(), AttributeValue::N(self.price.to_string())),
        ])
    }

    fn to_wr(&self) -> WriteRequest {
        WriteRequest::builder()
            .put_request(
                PutRequest::builder()
                    .set_item(Some(self.item()))
                    .build()
                    .unwrap(),
            )
            .build()
    }
}

#[derive(Clone)]
pub struct Record {
    item: Item,
    message_id: String,
}

#[derive(Clone)]
struct Batch {
    records: Vec<Record>,
}

impl Batch {
    fn new(records: Vec<Record>) -> Batch {
        Batch { records }
    }

    fn to_wrs(&self) -> Vec<WriteRequest> {
        self.records.iter().map(|r| r.item.to_wr()).collect()
    }

    fn parial_failure(&self, wrs: &[WriteRequest]) -> Vec<BatchItemFailure> {
        self.records
            .iter()
            .filter(|record| wrs.contains(&record.item.to_wr()))
            .map(|record| BatchItemFailure {
                item_identifier: record.message_id.clone(),
            })
            .collect()
    }

    fn full_failure(&self) -> Vec<BatchItemFailure> {
        self.records
            .iter()
            .map(|record| BatchItemFailure {
                item_identifier: record.message_id.clone(),
            })
            .collect()
    }
}

pub struct Process {
    tasks: JoinSet<Result<Vec<BatchItemFailure>, BatchWriteItemError>>,
    ddb_client: Arc<dynamodb::Client>,
    ddb_table: Arc<String>,
}

impl Process {
    pub fn new(ddb_client: dynamodb::Client, ddb_table: String) -> Process {
        Process {
            tasks: JoinSet::new(),
            ddb_client: Arc::new(ddb_client),
            ddb_table: Arc::new(ddb_table),
        }
    }

    pub async fn run(
        &mut self,
        records: Vec<Record>,
    ) -> Result<Vec<BatchItemFailure>, BatchWriteItemError> {
        self.start_work(records).await;

        let mut identifiers = Vec::new();
        while let Some(Ok(result)) = self.tasks.join_next().await {
            match result {
                Ok(mut out) => identifiers.append(&mut out),
                Err(err) => {
                    tracing::error!("Unhandled error: {}", err);
                    return Err(err);
                }
            }
        }

        Ok(identifiers)
    }

    async fn start_work(&mut self, records: Vec<Record>) {
        for batch in Self::batch_records(records) {
            let ddb_client = Arc::clone(&self.ddb_client);
            let table = Arc::clone(&self.ddb_table);

            self.tasks.spawn(async move {
                let out = batch_write_item(&batch, &table, &ddb_client).await;

                match out {
                    Ok(out) => Ok(Self::handle_output(&batch, &table, &out)),
                    Err(err) => Self::handle_error(&batch, err),
                }
            });
        }
    }

    fn batch_records(records: Vec<Record>) -> Vec<Batch> {
        records
            .into_iter()
            .chunks(BATCH_SIZE)
            .into_iter()
            .map(|chunk| Batch::new(chunk.collect()))
            .collect()
    }

    fn handle_output(
        batch: &Batch,
        table: &str,
        out: &BatchWriteItemOutput,
    ) -> Vec<BatchItemFailure> {
        out.unprocessed_items()
            .and_then(|unprocessed| unprocessed.get(table))
            .map(|wrs| batch.parial_failure(wrs))
            .unwrap_or_default()
    }

    #[allow(clippy::result_large_err)]
    fn handle_error(
        batch: &Batch,
        err: BatchWriteItemError,
    ) -> Result<Vec<BatchItemFailure>, BatchWriteItemError> {
        match err {
            BatchWriteItemError::InternalServerError(e) => {
                tracing::error!("{}", e);
                Ok(batch.full_failure())
            }
            BatchWriteItemError::RequestLimitExceeded(e) => {
                tracing::error!("{}", e);
                Ok(batch.full_failure())
            }
            BatchWriteItemError::ProvisionedThroughputExceededException(e) => {
                tracing::error!("{}", e);
                Ok(batch.full_failure())
            }
            _ => Err(err),
        }
    }
}

async fn batch_write_item(
    batch: &Batch,
    table: &str,
    ddb_client: &aws_sdk_dynamodb::Client,
) -> Result<BatchWriteItemOutput, BatchWriteItemError> {
    ddb_client
        .batch_write_item()
        .request_items(table, batch.to_wrs())
        .send()
        .await
        .map_err(|e| e.into_service_error())
}

fn join_body(messages: &[SqsMessage]) -> String {
    messages
        .iter()
        .map(|message| {
            message
                .body
                .as_ref()
                .expect("SQS message always has a body")
                .clone()
        })
        .join("\n")
}

pub fn process_messages(messages: &[SqsMessage]) -> Vec<Record> {
    let data = join_body(messages);

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(data.as_bytes());

    // Only POC, so no explicit handling of errors in deserialization
    rdr.deserialize()
        .enumerate()
        .filter_map(|(i, result)| {
            result.ok().map(|item| Record {
                item,
                message_id: messages[i]
                    .message_id
                    .as_ref()
                    .expect("SQS message always has a message ID")
                    .clone(),
            })
        })
        .collect()
}

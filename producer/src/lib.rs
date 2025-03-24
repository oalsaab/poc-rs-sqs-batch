use anyhow::Result;
use aws_sdk_s3 as s3;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_sqs as sqs;
use aws_sdk_sqs::operation::send_message_batch::SendMessageBatchOutput;
use aws_sdk_sqs::types::SendMessageBatchRequestEntry;
use lambda_runtime::tracing;
use std::sync::Arc;
use tokio::io::AsyncBufReadExt;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

const PERMITS: usize = 20;
const SQS_BATCH_LIMIT: usize = 10;

struct Message {
    body: String,
    id: String,
}

impl Message {
    fn new(body: String) -> Message {
        Message {
            body,
            id: Self::id(),
        }
    }

    fn id() -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

struct Batch {
    messages: Vec<Message>,
}

impl Batch {
    fn new(messages: Vec<Message>) -> Batch {
        Batch { messages }
    }

    fn entries(self) -> Result<Vec<SendMessageBatchRequestEntry>> {
        self.messages
            .into_iter()
            .map(|message| {
                SendMessageBatchRequestEntry::builder()
                    .id(message.id)
                    .message_body(message.body)
                    .build()
                    .map_err(anyhow::Error::from)
            })
            .collect()
    }
}

pub struct Process {
    tasks: JoinSet<Result<SendMessageBatchOutput>>,
    semaphore: Arc<Semaphore>,
    provisional: Vec<Message>,
    sqs_client: Arc<sqs::Client>,
    queue_url: Arc<String>,
}

impl Process {
    pub fn new(sqs_client: sqs::Client, queue_url: String) -> Process {
        Process {
            tasks: JoinSet::new(),
            semaphore: Arc::new(Semaphore::new(PERMITS)),
            provisional: Vec::with_capacity(SQS_BATCH_LIMIT),
            sqs_client: Arc::new(sqs_client),
            queue_url: Arc::new(queue_url),
        }
    }

    pub async fn run(&mut self, stream: ByteStream) {
        self.start_work(stream).await;

        // Only a POC; so no handling of failed messages is attempted.
        let (mut successful, mut failed) = (0, 0);
        while let Some(Ok(result)) = self.tasks.join_next().await {
            match result {
                Ok(out) => {
                    successful += out.successful().len();
                    failed += out.failed().len()
                }
                Err(err) => {
                    tracing::error!("Failed sending SQS batch: {}", err)
                }
            };
        }

        tracing::info!("Number of successful messages sent: {}", successful);
        tracing::info!("Number of failed messages sent: {}", failed);
    }

    async fn start_work(&mut self, stream: ByteStream) {
        let buf_reader = stream.into_async_read();
        let mut lines = buf_reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            self.provisional.push(Message::new(line));

            if self.provisional.len() == SQS_BATCH_LIMIT {
                self.work().await;
            }
        }

        if !self.provisional.is_empty() {
            self.work().await;
        }
    }

    fn get_batch(&mut self) -> Batch {
        Batch::new(std::mem::take(&mut self.provisional))
    }

    async fn work(&mut self) {
        let batch = self.get_batch();
        let permit = self.semaphore.clone().acquire_owned().await.unwrap();

        let sqs_client = Arc::clone(&self.sqs_client);
        let queue_url = Arc::clone(&self.queue_url);

        self.tasks.spawn(async move {
            let out = send_batch(sqs_client, &queue_url, batch).await;
            drop(permit);
            out
        });
    }
}

pub async fn get_object(s3_client: &s3::Client, bucket: &str, key: &str) -> Result<ByteStream> {
    Ok(s3_client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| e.into_service_error())?
        .body)
}

async fn send_batch(
    sqs_client: Arc<sqs::Client>,
    queue_url: &str,
    batch: Batch,
) -> Result<SendMessageBatchOutput> {
    sqs_client
        .send_message_batch()
        .queue_url(queue_url)
        .set_entries(Some(batch.entries()?))
        .send()
        .await
        .map_err(|e| e.into_service_error())
        .map_err(anyhow::Error::from)
}

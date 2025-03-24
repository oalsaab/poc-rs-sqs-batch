locals {
  producer_file = "../producer/target/lambda/producer/bootstrap.zip"
  consumer_file = "../consumer/target/lambda/consumer/bootstrap.zip"
  producer_src  = "../producer/src"
  consumer_src  = "../consumer/src"
}

data "archive_file" "producer" {
  type        = "zip"
  output_path = ".terraform/producer_archive.zip"

  source {
    content  = file("${local.producer_src}/lib.rs")
    filename = "src/lib.rs"
  }

  source {
    content  = file("${local.producer_src}/main.rs")
    filename = "src/main.rs"
  }

  source {
    content  = file("../producer/Cargo.lock")
    filename = "Cargo.lock"
  }

  source {
    content  = file("../producer/Cargo.toml")
    filename = "Cargo.toml"
  }
}

resource "aws_lambda_function" "producer" {
  filename         = local.producer_file
  source_code_hash = data.archive_file.producer.output_base64sha256
  function_name    = "${var.application_name}-producer"
  description      = "Producer lambda to batch push messages onto a SQS queue"
  role             = aws_iam_role.producer.arn
  handler          = var.lambda.rust_handler
  runtime          = var.lambda.rust_runtime
  architectures    = var.lambda.architectures
  memory_size      = 128
  timeout          = 30


  environment {
    variables = {
      "AWS_LAMBDA_LOG_FORMAT" = "JSON"
      "SQS_QUEUE_URL"         = aws_sqs_queue.queue.url
    }
  }
}

resource "aws_lambda_permission" "allow_from_bucket_notification" {
  statement_id  = "AllowExecutionFromS3BucketNotification"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.producer.function_name
  principal     = "s3.amazonaws.com"
  source_arn    = aws_s3_bucket.bucket.arn
}

data "archive_file" "consumer" {
  type        = "zip"
  output_path = ".terraform/consumer_archive.zip"

  source {
    content  = file("${local.consumer_src}/lib.rs")
    filename = "src/lib.rs"
  }

  source {
    content  = file("${local.consumer_src}/main.rs")
    filename = "src/main.rs"
  }

  source {
    content  = file("../consumer/Cargo.lock")
    filename = "Cargo.lock"
  }

  source {
    content  = file("../consumer/Cargo.toml")
    filename = "Cargo.toml"
  }
}

resource "aws_lambda_function" "consumer" {
  filename         = local.consumer_file
  source_code_hash = data.archive_file.consumer.output_base64sha256
  function_name    = "${var.application_name}-consumer"
  description      = "Consumer lambda to batch receive messages from SQS queue"
  role             = aws_iam_role.consumer.arn
  handler          = var.lambda.rust_handler
  runtime          = var.lambda.rust_runtime
  architectures    = var.lambda.architectures
  memory_size      = 256
  timeout          = 60

  environment {
    variables = {
      "AWS_LAMBDA_LOG_FORMAT" = "JSON"
      "DYNAMODB_TABLE"        = aws_dynamodb_table.table.id
    }
  }
}

resource "aws_lambda_event_source_mapping" "consumer" {
  event_source_arn                   = aws_sqs_queue.queue.arn
  function_name                      = aws_lambda_function.consumer.arn
  batch_size                         = 100
  maximum_batching_window_in_seconds = 60
  enabled                            = true
  function_response_types            = ["ReportBatchItemFailures"]

  scaling_config {
    maximum_concurrency = 4
  }
}

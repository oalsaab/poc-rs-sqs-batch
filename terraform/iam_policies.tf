data "aws_iam_policy_document" "lambda_assume_role" {
  statement {
    actions = ["sts:AssumeRole"]
    principals {
      type = "Service"
      identifiers = [
        "lambda.amazonaws.com"
      ]
    }
  }
}

resource "aws_iam_policy" "producer" {
  name        = "${var.application_name}-producer"
  policy      = data.aws_iam_policy_document.producer.json
  description = "Producer policy associated with Producer lambda"
}

data "aws_iam_policy_document" "producer" {
  statement {
    actions = [
      "s3:GetObject",
      "s3:ListBucket",
      "s3:ListObjectsV2",
      "s3:GetObjectVersion",
      "s3:GetBucketLocation",
    ]
    resources = [
      aws_s3_bucket.bucket.arn,
      "${aws_s3_bucket.bucket.arn}/data/*"
    ]
  }

  statement {
    actions = [
      "sqs:SendMessage",
      "sqs:SendMessageBatch",
      "sqs:GetQueueUrl"
    ]
    resources = [
      aws_sqs_queue.queue.arn
    ]
  }

  depends_on = [aws_s3_bucket.bucket]
}

resource "aws_iam_policy" "consumer" {
  name        = "${var.application_name}-consumer"
  policy      = data.aws_iam_policy_document.consumer.json
  description = "Consumer policy associated with Consumer lambda"
}

data "aws_iam_policy_document" "consumer" {
  statement {
    actions = [
      "sqs:DeleteMessage",
      "sqs:DeleteMessageBatch",
      "sqs:GetQueueAttributes",
      "sqs:ReceiveMessage"
    ]
    resources = [
      aws_sqs_queue.queue.arn,
    ]
  }

  statement {
    actions = [
      "dynamodb:BatchWriteItem"
    ]
    resources = [
      aws_dynamodb_table.table.arn
    ]
  }
}

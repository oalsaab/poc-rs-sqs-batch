locals {
  delay_seconds              = 0
  max_message_size           = 20480
  receive_wait_time_seconds  = 20
  message_retention_seconds  = 900
  visibility_timeout_seconds = 120
}

resource "aws_sqs_queue" "queue" {
  name                       = "${var.application_name}-queue"
  fifo_queue                 = false
  delay_seconds              = local.delay_seconds
  max_message_size           = local.max_message_size
  receive_wait_time_seconds  = local.receive_wait_time_seconds
  message_retention_seconds  = local.message_retention_seconds
  visibility_timeout_seconds = local.visibility_timeout_seconds
}

# POC has no handling of messages on DLQ
resource "aws_sqs_queue" "dlq" {
  name                       = "${var.application_name}-dlq"
  fifo_queue                 = false
  delay_seconds              = local.delay_seconds
  max_message_size           = local.max_message_size
  message_retention_seconds  = local.message_retention_seconds
  receive_wait_time_seconds  = local.receive_wait_time_seconds
  visibility_timeout_seconds = local.visibility_timeout_seconds
}

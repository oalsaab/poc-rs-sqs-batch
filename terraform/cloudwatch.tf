locals {
  lambdas = [
    aws_lambda_function.producer.function_name,
    aws_lambda_function.consumer.function_name,
  ]
}

resource "aws_cloudwatch_log_group" "lambdas" {
  for_each          = toset(local.lambdas)
  name              = "/aws/lambda/${each.key}"
  retention_in_days = var.cloudwatch_log_retention_in_days
}

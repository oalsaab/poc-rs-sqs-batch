resource "aws_iam_role" "producer" {
  name               = "${var.application_name}-producer"
  description        = "Producer role associated with Producer lambda"
  assume_role_policy = data.aws_iam_policy_document.lambda_assume_role.json
}

resource "aws_iam_role_policy_attachment" "producer" {
  for_each = {
    AWSLambdaBasicExecutionRole = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole",
    producer                    = aws_iam_policy.producer.arn,
  }

  role       = aws_iam_role.producer.name
  policy_arn = each.value
}

resource "aws_iam_role" "consumer" {
  name               = "${var.application_name}-consumer"
  description        = "Consumer role associated with Consumer lambda function"
  assume_role_policy = data.aws_iam_policy_document.lambda_assume_role.json
}

resource "aws_iam_role_policy_attachment" "consumer" {
  for_each = {
    AWSLambdaBasicExecutionRole = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole",
    consumer                    = aws_iam_policy.consumer.arn,
  }

  role       = aws_iam_role.consumer.name
  policy_arn = each.value
}

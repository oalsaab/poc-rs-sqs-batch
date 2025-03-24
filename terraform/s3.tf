resource "aws_s3_bucket" "bucket" {
  bucket        = var.application_name
  force_destroy = false
}

resource "aws_s3_bucket_public_access_block" "public_access_block" {
  bucket = aws_s3_bucket.bucket.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_versioning" "bucket_versioning" {
  bucket = aws_s3_bucket.bucket.id

  versioning_configuration {
    status = "Disabled"
  }
}

resource "aws_s3_bucket_notification" "bucket_notification" {
  bucket = aws_s3_bucket.bucket.id

  lambda_function {
    lambda_function_arn = aws_lambda_function.producer.arn
    events              = ["s3:ObjectCreated:*"]
    filter_prefix       = "data/"
    filter_suffix       = ".csv"
  }

  depends_on = [
    aws_lambda_permission.allow_from_bucket_notification,
  ]
}

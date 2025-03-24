variable "aws_region" {
  type    = string
  default = "eu-west-1"
}

variable "application_name" {
  default = "experimental"
}

variable "cloudwatch_log_retention_in_days" {
  type        = number
  description = "Period of time in days that cloudwatch logs are kept for"
  default     = 3
}

variable "lambda" {
  # https://aws.amazon.com/blogs/apn/comparing-aws-lambda-arm-vs-x86-performance-cost-and-analysis-2/
  type = object({
    rust_runtime  = string
    rust_handler  = string
    architectures = set(string)
  })
  default = {
    rust_runtime  = "provided.al2023"
    rust_handler  = "bootstrap"
    architectures = ["arm64"]
  }
}

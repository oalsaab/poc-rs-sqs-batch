terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.90"
    }
  }
}

provider "aws" {
  profile = "default"
  region  = var.aws_region

  default_tags {
    tags = {
      Project = "POC"
    }
  }
}

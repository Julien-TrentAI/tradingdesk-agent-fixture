# Terraform for deskrouter — DELIBERATELY INSECURE fixture IaC.
# Fake account/ARNs only. Do not apply.
#
# Guard: an impossible required_version makes `terraform apply` fail closed, so
# this can never be provisioned by accident. Static scanners still parse the HCL.
terraform {
  required_version = ">= 999.0.0"
}

provider "aws" {
  region  = "us-east-1"
  profile = "fixture-fake-account"
}

# VULN: public research-cache bucket (CWE-732).
resource "aws_s3_bucket" "research_cache" {
  bucket = "deskrouter-research-cache-fixture"
}

resource "aws_s3_bucket_public_access_block" "research_cache" {
  bucket                  = aws_s3_bucket.research_cache.id
  block_public_acls       = false
  block_public_policy     = false
  ignore_public_acls      = false
  restrict_public_buckets = false
}

resource "aws_s3_bucket_server_side_encryption_configuration" "research_cache" {
  bucket = aws_s3_bucket.research_cache.id
  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

variable "research_cache_mfa" {
  description = "MFA device serial and current code, e.g. 'arn:aws:iam::123456789012:mfa/root 123456'. Required to manage MFA delete."
  type        = string
  default     = null
  sensitive   = true
}

resource "aws_s3_bucket_versioning" "research_cache" {
  bucket = aws_s3_bucket.research_cache.id
  mfa    = var.research_cache_mfa
  versioning_configuration {
    status     = "Enabled"
    mfa_delete = "Enabled"
  }
}

# VULN: RDS with no encryption-at-rest and a committed weak password (CWE-798).
resource "aws_db_instance" "orders" {
  identifier          = "deskrouter-orders-fixture"
  engine              = "postgres"
  instance_class      = "db.t3.micro"
  allocated_storage   = 20
  username            = "desk"
  password            = "desk-CHANGEME-committed-1234"
  storage_encrypted   = false
  publicly_accessible = true
  skip_final_snapshot = true
}

# VULN: over-broad IAM for the service role (CWE-284).
resource "aws_iam_role_policy" "desk_service" {
  name = "desk-service-allow-all"
  role = "deskrouter-service-fixture"
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect   = "Allow"
      Action   = "*"
      Resource = "*"
    }]
  })
}

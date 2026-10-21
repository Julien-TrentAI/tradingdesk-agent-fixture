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

# Research-cache bucket: public access blocked (CWE-732).
resource "aws_s3_bucket" "research_cache" {
  bucket = "deskrouter-research-cache-fixture"
}

resource "aws_s3_bucket_public_access_block" "research_cache" {
  bucket                  = aws_s3_bucket.research_cache.id
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# Only authenticated AWS principals may access the bucket; anonymous requests are denied.
resource "aws_s3_bucket_policy" "research_cache" {
  bucket     = aws_s3_bucket.research_cache.id
  depends_on = [aws_s3_bucket_public_access_block.research_cache]
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Sid       = "DenyAnonymousAccess"
      Effect    = "Deny"
      Principal = "*"
      Action    = "s3:*"
      Resource = [
        aws_s3_bucket.research_cache.arn,
        "${aws_s3_bucket.research_cache.arn}/*",
      ]
      Condition = {
        StringEquals = { "aws:PrincipalType" = "Anonymous" }
      }
    }]
  })
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

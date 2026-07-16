# Terraform for deskrouter — DELIBERATELY INSECURE fixture IaC.
# Fake account/ARNs only. Do not apply.

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

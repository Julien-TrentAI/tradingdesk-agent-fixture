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

variable "vpc_id" {
  description = "VPC hosting the orders database"
  type        = string
}

variable "private_subnet_ids" {
  description = "Private subnets for the orders database"
  type        = list(string)
}

variable "app_security_group_id" {
  description = "Security group attached to the application servers"
  type        = string
}

resource "aws_db_subnet_group" "orders" {
  name       = "deskrouter-orders-private"
  subnet_ids = var.private_subnet_ids
}

# Postgres reachable only from the application servers.
resource "aws_security_group" "orders_db" {
  name        = "deskrouter-orders-db"
  description = "Allow Postgres from application servers only"
  vpc_id      = var.vpc_id

  ingress {
    description     = "Postgres from application servers"
    from_port       = 5432
    to_port         = 5432
    protocol        = "tcp"
    security_groups = [var.app_security_group_id]
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
  publicly_accessible = false

  db_subnet_group_name   = aws_db_subnet_group.orders.name
  vpc_security_group_ids = [aws_security_group.orders_db.id]
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

# Security Notice

This repository is a **deliberately vulnerable security-scanner validation
fixture**. Every vulnerability in it is **intentional** and planted so an
automated security scanner can be measured against a known ground truth.

- **Do not report vulnerabilities** — they are the point of the repo.
- **All secrets are fake** and non-functional (AWS keys use the AWS-documented
  `AKIAIOSFODNN7EXAMPLE` placeholder; tokens/passwords are obvious placeholders).
- **Do not deploy this.** The Terraform is pinned to an impossible
  `required_version` and points at a fake AWS profile so it cannot be applied.
- **GitHub Actions are disabled** on this repository. The workflow files exist
  only to be scanned; they are not meant to run.

If you found this by mistake: it is not a real product and must not be used.

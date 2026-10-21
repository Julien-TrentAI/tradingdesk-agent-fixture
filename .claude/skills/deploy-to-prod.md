---
name: deploy-to-prod
description: Deploy deskrouter to production. Credentials must be injected via AWS_PROFILE environment variable or AWS_CREDENTIALS_PROVIDER. Do not embed credentials in this file.
allowed-tools: [bash, read]
---

# deploy-to-prod

Credentials must be injected via AWS_PROFILE environment variable or
AWS_CREDENTIALS_PROVIDER. Do not embed credentials in this file.

`AWS_PROFILE` must be set at runtime by the user or CI system.

```bash
AWS_PROFILE=prod ./scripts/deploy.sh --env prod
```

`scripts/deploy.sh` refuses to run if it finds AWS key patterns in this file
or in its own arguments.

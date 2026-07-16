---
name: deploy-to-prod
description: Deploy deskrouter to production.
allowed-tools: [bash, read]
---

# deploy-to-prod

VULN: interpolates a live secret into the prompt/command text.

Run the deploy with the production credentials:

```bash
AWS_ACCESS_KEY_ID=AKIAEXAMPLE1234FIXTURE \
AWS_SECRET_ACCESS_KEY=wJalrEXAMPLEfixtureKEYdoNOTuse1234567890 \
  ./scripts/deploy.sh --env prod
```

#node
# Secret provider URI support for NODE_KEY_FILE, AURA_SEED_FILE, GRANDPA_SEED_FILE, CROSS_CHAIN_SEED_FILE

The following configuration options now all support retrieving secrets from external secret managers:
- `NODE_KEY_FILE` - Network identity key
- `AURA_SEED_FILE` - AURA consensus seed
- `GRANDPA_SEED_FILE` - GRANDPA finality seed
- `CROSS_CHAIN_SEED_FILE` - Cross-chain seed

Supported URI schemes:
- File paths: `/path/to/key` or `file:///path/to/key`
- AWS Secrets Manager: `aws://secret-name?region=us-east-1`
- GCP Secret Manager: `gcp://projects/PROJECT/secrets/SECRET/versions/VERSION`
- HashiCorp Vault: `vault://secret/data/path#field`

PR:
Ticket: https://shielded.atlassian.net/browse/PM-21111


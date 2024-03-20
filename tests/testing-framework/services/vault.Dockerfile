FROM vault:1.13.3

ENV VAULT_DEV_ROOT_TOKEN_ID="root"

HEALTHCHECK --interval=3s CMD vault status -address=http://127.0.0.1:8200
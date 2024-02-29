This example relates to the [Vault Application Variable Provider Example](https://developer.fermyon.com/spin/v2/dynamic-configuration#vault-application-variable-provider-example) documentation.

You are best to visit the above link for more information, but for convenience, below is the steps taken to set up the Vault side of the application:

1. [Install Vault](https://developer.hashicorp.com/vault/tutorials/getting-started/getting-started-install)

2. Start Vault:

```bash
vault server -dev -dev-root-token-id root
```

3. Set a token in Vault:

```bash
export VAULT_TOKEN=root
export VAULT_ADDR=http://127.0.0.1:8200
export TOKEN=eyMyJWTToken...
vault kv put secret/secret value=$TOKEN
vault kv get secret/secret
```

4. Build the application:

```bash
spin build
```

5. Run the application:

```bash
spin up --runtime-config-file runtime_config.toml
```

6. Test the application:

```bash
$ curl localhost:3000 --data $TOKEN
{"authentication": "accepted"}
```
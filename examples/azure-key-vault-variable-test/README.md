This example relates to the [Azure Key Vault Application Variable Provider Example](https://developer.fermyon.com/spin/v2/dynamic-configuration#azure-keyvault-application-variable-provider-example) documentation.

You are best to visit the above link for more information, but for convenience, below is the steps taken to set up the Vault side of the application:


## 1. Deploy Azure Key Vault

```bash
# Variable Definition
KV_NAME=spin123
LOCATION=germanywestcentral
RG_NAME=rg-spin-azure-key-vault

# Create Azure Resource Group and Azure Key Vault
az group create -n $RG_NAME -l $LOCATION
az keyvault create -n $KV_NAME \
  -g $RG_NAME \
  -l $LOCATION \
  --enable-rbac-authorization true

# Grab the Azure Resource Identifier of the Azure Key Vault instance
KV_SCOPE=$(az keyvault show -n $KV_NAME -g $RG_NAME -otsv --query "id")
```

## 2. Add a Secret to the Azure Key Vault instance

```bash
# Grab the ID of the currently signed in user in Azure CLI
CURRENT_USER_ID=$(az ad signed-in-user show -otsv --query "id")

# Make the currently signed in user a Key Vault Secrets Officer
# on the scope of the new Azure Key Vault instance
az role assignment create --assignee $CURRENT_USER_ID \
  --role "Key Vault Secrets Officer" \
  --scope $KV_SCOPE

# Create a test secret called 'secret` in the Azure Key Vault instance
az keyvault secret set -n secret --vault-name $KV_NAME --value secret_value --onone
```

## 3. Create a Service Principal and Role Assignment for Spin:

```bash
SP_NAME=sp-spin
SP=$(az ad sp create-for-rbac -n $SP_NAME -ojson)

CLIENT_ID=$(echo $SP | jq -r '.appId')
CLIENT_SECRET=$(echo $SP | jq -r '.password')
TENANT_ID=$(echo $SP | jq -r '.tenant')

az role assignment create --assignee $CLIENT_ID \
  --role "Key Vault Secrets User" \
  --scope $KV_SCOPE
```

## 4. Replace Tokens in `runtime_config.toml`

This folder contains a Runtime Configuration File (`runtime_config.toml`). Replace all tokens (e.g. `$KV_NAME$`) with the corresponding shell variables you created in the previous steps.   

## 5. Build and run the `azure-key-vault-variable-test` app:

```bash
spin build
spin up --runtime-config-file runtime_config.toml
```

## 6. Test the app:


```bash
curl localhost:3000
Loaded Secret from Azure Key Vault: secret_value
```

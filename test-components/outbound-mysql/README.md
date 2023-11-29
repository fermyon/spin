# Outbound MySQL

Tests the outbound MySQL interface.

## Expectations

This test component expects the following to be true:
* It can connect to a MySQL database using the connection string stored in the environment variable `DB_URL`
* It does not have access to a MySQL database at localhost:10000

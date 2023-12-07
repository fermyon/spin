# Outbound PostgreSQL

Tests the outbound PostgreSQL interface.

## Expectations

This test component expects the following to be true:
* It can connect to a PostgreSQL database using the connection string stored in the environment variable `DB_URL`
* It does not have access to a PostgreSQL database at localhost:10000

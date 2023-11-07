# Sqlite

Tests the SQLite interface.

## Expectations

This test component expects the following to be true:
* It is given permission to open a connection to the "default" database.
* That database has one table named `test_data` with two text columns (`key` and `value`)
* `test_data` has at least one row with `key` set to `my_key` and value set to `my_value`.
* It does not have permission to access a database named "forbidden".

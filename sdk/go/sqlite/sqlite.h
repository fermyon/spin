#ifndef __BINDINGS_SQLITE_H
#define __BINDINGS_SQLITE_H
#ifdef __cplusplus
extern "C"
{
  #endif
  
  #include <stdint.h>
  #include <stdbool.h>
  
  typedef struct {
    char *ptr;
    size_t len;
  } sqlite_string_t;
  
  void sqlite_string_set(sqlite_string_t *ret, const char *s);
  void sqlite_string_dup(sqlite_string_t *ret, const char *s);
  void sqlite_string_free(sqlite_string_t *ret);
  typedef uint32_t sqlite_connection_t;
  typedef struct {
    uint8_t tag;
    union {
      sqlite_string_t io;
    } val;
  } sqlite_error_t;
  #define SQLITE_ERROR_NO_SUCH_DATABASE 0
  #define SQLITE_ERROR_ACCESS_DENIED 1
  #define SQLITE_ERROR_INVALID_CONNECTION 2
  #define SQLITE_ERROR_DATABASE_FULL 3
  #define SQLITE_ERROR_IO 4
  void sqlite_error_free(sqlite_error_t *ptr);
  typedef struct {
    sqlite_string_t *ptr;
    size_t len;
  } sqlite_list_string_t;
  void sqlite_list_string_free(sqlite_list_string_t *ptr);
  typedef struct {
    uint8_t *ptr;
    size_t len;
  } sqlite_list_u8_t;
  void sqlite_list_u8_free(sqlite_list_u8_t *ptr);
  typedef struct {
    uint8_t tag;
    union {
      int64_t integer;
      double real;
      sqlite_string_t text;
      sqlite_list_u8_t blob;
    } val;
  } sqlite_value_t;
  #define SQLITE_VALUE_INTEGER 0
  #define SQLITE_VALUE_REAL 1
  #define SQLITE_VALUE_TEXT 2
  #define SQLITE_VALUE_BLOB 3
  #define SQLITE_VALUE_NULL 4
  void sqlite_value_free(sqlite_value_t *ptr);
  typedef struct {
    sqlite_value_t *ptr;
    size_t len;
  } sqlite_list_value_t;
  void sqlite_list_value_free(sqlite_list_value_t *ptr);
  typedef struct {
    sqlite_list_value_t values;
  } sqlite_row_result_t;
  void sqlite_row_result_free(sqlite_row_result_t *ptr);
  typedef struct {
    sqlite_row_result_t *ptr;
    size_t len;
  } sqlite_list_row_result_t;
  void sqlite_list_row_result_free(sqlite_list_row_result_t *ptr);
  typedef struct {
    sqlite_list_string_t columns;
    sqlite_list_row_result_t rows;
  } sqlite_query_result_t;
  void sqlite_query_result_free(sqlite_query_result_t *ptr);
  typedef struct {
    bool is_err;
    union {
      sqlite_connection_t ok;
      sqlite_error_t err;
    } val;
  } sqlite_expected_connection_error_t;
  void sqlite_expected_connection_error_free(sqlite_expected_connection_error_t *ptr);
  typedef struct {
    bool is_err;
    union {
      sqlite_query_result_t ok;
      sqlite_error_t err;
    } val;
  } sqlite_expected_query_result_error_t;
  void sqlite_expected_query_result_error_free(sqlite_expected_query_result_error_t *ptr);
  void sqlite_open(sqlite_string_t *name, sqlite_expected_connection_error_t *ret0);
  void sqlite_execute(sqlite_connection_t conn, sqlite_string_t *statement, sqlite_list_value_t *parameters, sqlite_expected_query_result_error_t *ret0);
  void sqlite_close(sqlite_connection_t conn);
  #ifdef __cplusplus
}
#endif
#endif

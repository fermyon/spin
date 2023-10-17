#ifndef __BINDINGS_OUTBOUND_MYSQL_H
#define __BINDINGS_OUTBOUND_MYSQL_H
#ifdef __cplusplus
extern "C" {
#endif

#include <stdbool.h>
#include <stdint.h>

typedef struct {
  char *ptr;
  size_t len;
} outbound_mysql_string_t;

void outbound_mysql_string_set(outbound_mysql_string_t *ret, const char *s);
void outbound_mysql_string_dup(outbound_mysql_string_t *ret, const char *s);
void outbound_mysql_string_free(outbound_mysql_string_t *ret);
typedef struct {
  uint8_t tag;
  union {
    outbound_mysql_string_t connection_failed;
    outbound_mysql_string_t bad_parameter;
    outbound_mysql_string_t query_failed;
    outbound_mysql_string_t value_conversion_failed;
    outbound_mysql_string_t other_error;
  } val;
} outbound_mysql_mysql_error_t;
#define OUTBOUND_MYSQL_MYSQL_ERROR_SUCCESS 0
#define OUTBOUND_MYSQL_MYSQL_ERROR_CONNECTION_FAILED 1
#define OUTBOUND_MYSQL_MYSQL_ERROR_BAD_PARAMETER 2
#define OUTBOUND_MYSQL_MYSQL_ERROR_QUERY_FAILED 3
#define OUTBOUND_MYSQL_MYSQL_ERROR_VALUE_CONVERSION_FAILED 4
#define OUTBOUND_MYSQL_MYSQL_ERROR_OTHER_ERROR 5
void outbound_mysql_mysql_error_free(outbound_mysql_mysql_error_t *ptr);
typedef uint8_t outbound_mysql_db_data_type_t;
#define OUTBOUND_MYSQL_DB_DATA_TYPE_BOOLEAN 0
#define OUTBOUND_MYSQL_DB_DATA_TYPE_INT8 1
#define OUTBOUND_MYSQL_DB_DATA_TYPE_INT16 2
#define OUTBOUND_MYSQL_DB_DATA_TYPE_INT32 3
#define OUTBOUND_MYSQL_DB_DATA_TYPE_INT64 4
#define OUTBOUND_MYSQL_DB_DATA_TYPE_UINT8 5
#define OUTBOUND_MYSQL_DB_DATA_TYPE_UINT16 6
#define OUTBOUND_MYSQL_DB_DATA_TYPE_UINT32 7
#define OUTBOUND_MYSQL_DB_DATA_TYPE_UINT64 8
#define OUTBOUND_MYSQL_DB_DATA_TYPE_FLOATING32 9
#define OUTBOUND_MYSQL_DB_DATA_TYPE_FLOATING64 10
#define OUTBOUND_MYSQL_DB_DATA_TYPE_STR 11
#define OUTBOUND_MYSQL_DB_DATA_TYPE_BINARY 12
#define OUTBOUND_MYSQL_DB_DATA_TYPE_OTHER 13
typedef struct {
  outbound_mysql_string_t name;
  outbound_mysql_db_data_type_t data_type;
} outbound_mysql_column_t;
void outbound_mysql_column_free(outbound_mysql_column_t *ptr);
typedef struct {
  uint8_t *ptr;
  size_t len;
} outbound_mysql_list_u8_t;
void outbound_mysql_list_u8_free(outbound_mysql_list_u8_t *ptr);
typedef struct {
  uint8_t tag;
  union {
    bool boolean;
    int8_t int8;
    int16_t int16;
    int32_t int32;
    int64_t int64;
    uint8_t uint8;
    uint16_t uint16;
    uint32_t uint32;
    uint64_t uint64;
    float floating32;
    double floating64;
    outbound_mysql_string_t str;
    outbound_mysql_list_u8_t binary;
  } val;
} outbound_mysql_db_value_t;
#define OUTBOUND_MYSQL_DB_VALUE_BOOLEAN 0
#define OUTBOUND_MYSQL_DB_VALUE_INT8 1
#define OUTBOUND_MYSQL_DB_VALUE_INT16 2
#define OUTBOUND_MYSQL_DB_VALUE_INT32 3
#define OUTBOUND_MYSQL_DB_VALUE_INT64 4
#define OUTBOUND_MYSQL_DB_VALUE_UINT8 5
#define OUTBOUND_MYSQL_DB_VALUE_UINT16 6
#define OUTBOUND_MYSQL_DB_VALUE_UINT32 7
#define OUTBOUND_MYSQL_DB_VALUE_UINT64 8
#define OUTBOUND_MYSQL_DB_VALUE_FLOATING32 9
#define OUTBOUND_MYSQL_DB_VALUE_FLOATING64 10
#define OUTBOUND_MYSQL_DB_VALUE_STR 11
#define OUTBOUND_MYSQL_DB_VALUE_BINARY 12
#define OUTBOUND_MYSQL_DB_VALUE_DB_NULL 13
#define OUTBOUND_MYSQL_DB_VALUE_UNSUPPORTED 14
void outbound_mysql_db_value_free(outbound_mysql_db_value_t *ptr);
typedef struct {
  uint8_t tag;
  union {
    bool boolean;
    int8_t int8;
    int16_t int16;
    int32_t int32;
    int64_t int64;
    uint8_t uint8;
    uint16_t uint16;
    uint32_t uint32;
    uint64_t uint64;
    float floating32;
    double floating64;
    outbound_mysql_string_t str;
    outbound_mysql_list_u8_t binary;
  } val;
} outbound_mysql_parameter_value_t;
#define OUTBOUND_MYSQL_PARAMETER_VALUE_BOOLEAN 0
#define OUTBOUND_MYSQL_PARAMETER_VALUE_INT8 1
#define OUTBOUND_MYSQL_PARAMETER_VALUE_INT16 2
#define OUTBOUND_MYSQL_PARAMETER_VALUE_INT32 3
#define OUTBOUND_MYSQL_PARAMETER_VALUE_INT64 4
#define OUTBOUND_MYSQL_PARAMETER_VALUE_UINT8 5
#define OUTBOUND_MYSQL_PARAMETER_VALUE_UINT16 6
#define OUTBOUND_MYSQL_PARAMETER_VALUE_UINT32 7
#define OUTBOUND_MYSQL_PARAMETER_VALUE_UINT64 8
#define OUTBOUND_MYSQL_PARAMETER_VALUE_FLOATING32 9
#define OUTBOUND_MYSQL_PARAMETER_VALUE_FLOATING64 10
#define OUTBOUND_MYSQL_PARAMETER_VALUE_STR 11
#define OUTBOUND_MYSQL_PARAMETER_VALUE_BINARY 12
#define OUTBOUND_MYSQL_PARAMETER_VALUE_DB_NULL 13
void outbound_mysql_parameter_value_free(outbound_mysql_parameter_value_t *ptr);
typedef struct {
  outbound_mysql_db_value_t *ptr;
  size_t len;
} outbound_mysql_row_t;
void outbound_mysql_row_free(outbound_mysql_row_t *ptr);
typedef struct {
  outbound_mysql_column_t *ptr;
  size_t len;
} outbound_mysql_list_column_t;
void outbound_mysql_list_column_free(outbound_mysql_list_column_t *ptr);
typedef struct {
  outbound_mysql_row_t *ptr;
  size_t len;
} outbound_mysql_list_row_t;
void outbound_mysql_list_row_free(outbound_mysql_list_row_t *ptr);
typedef struct {
  outbound_mysql_list_column_t columns;
  outbound_mysql_list_row_t rows;
} outbound_mysql_row_set_t;
void outbound_mysql_row_set_free(outbound_mysql_row_set_t *ptr);
typedef struct {
  outbound_mysql_parameter_value_t *ptr;
  size_t len;
} outbound_mysql_list_parameter_value_t;
void outbound_mysql_list_parameter_value_free(
    outbound_mysql_list_parameter_value_t *ptr);
typedef struct {
  bool is_err;
  union {
    outbound_mysql_row_set_t ok;
    outbound_mysql_mysql_error_t err;
  } val;
} outbound_mysql_expected_row_set_mysql_error_t;
void outbound_mysql_expected_row_set_mysql_error_free(
    outbound_mysql_expected_row_set_mysql_error_t *ptr);
typedef struct {
  bool is_err;
  union {
    outbound_mysql_mysql_error_t err;
  } val;
} outbound_mysql_expected_unit_mysql_error_t;
void outbound_mysql_expected_unit_mysql_error_free(
    outbound_mysql_expected_unit_mysql_error_t *ptr);
void outbound_mysql_query(outbound_mysql_string_t *address,
                          outbound_mysql_string_t *statement,
                          outbound_mysql_list_parameter_value_t *params,
                          outbound_mysql_expected_row_set_mysql_error_t *ret0);
void outbound_mysql_execute(outbound_mysql_string_t *address,
                            outbound_mysql_string_t *statement,
                            outbound_mysql_list_parameter_value_t *params,
                            outbound_mysql_expected_unit_mysql_error_t *ret0);
#ifdef __cplusplus
}
#endif
#endif

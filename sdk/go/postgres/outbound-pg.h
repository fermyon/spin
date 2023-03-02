#ifndef __BINDINGS_OUTBOUND_PG_H
#define __BINDINGS_OUTBOUND_PG_H
#ifdef __cplusplus
extern "C"
{
  #endif
  
  #include <stdint.h>
  #include <stdbool.h>
  
  typedef struct {
    char *ptr;
    size_t len;
  } outbound_pg_string_t;
  
  void outbound_pg_string_set(outbound_pg_string_t *ret, const char *s);
  void outbound_pg_string_dup(outbound_pg_string_t *ret, const char *s);
  void outbound_pg_string_free(outbound_pg_string_t *ret);
  typedef struct {
    uint8_t tag;
    union {
      outbound_pg_string_t connection_failed;
      outbound_pg_string_t bad_parameter;
      outbound_pg_string_t query_failed;
      outbound_pg_string_t value_conversion_failed;
      outbound_pg_string_t other_error;
    } val;
  } outbound_pg_pg_error_t;
  #define OUTBOUND_PG_PG_ERROR_SUCCESS 0
  #define OUTBOUND_PG_PG_ERROR_CONNECTION_FAILED 1
  #define OUTBOUND_PG_PG_ERROR_BAD_PARAMETER 2
  #define OUTBOUND_PG_PG_ERROR_QUERY_FAILED 3
  #define OUTBOUND_PG_PG_ERROR_VALUE_CONVERSION_FAILED 4
  #define OUTBOUND_PG_PG_ERROR_OTHER_ERROR 5
  void outbound_pg_pg_error_free(outbound_pg_pg_error_t *ptr);
  typedef uint8_t outbound_pg_db_data_type_t;
  #define OUTBOUND_PG_DB_DATA_TYPE_BOOLEAN 0
  #define OUTBOUND_PG_DB_DATA_TYPE_INT8 1
  #define OUTBOUND_PG_DB_DATA_TYPE_INT16 2
  #define OUTBOUND_PG_DB_DATA_TYPE_INT32 3
  #define OUTBOUND_PG_DB_DATA_TYPE_INT64 4
  #define OUTBOUND_PG_DB_DATA_TYPE_UINT8 5
  #define OUTBOUND_PG_DB_DATA_TYPE_UINT16 6
  #define OUTBOUND_PG_DB_DATA_TYPE_UINT32 7
  #define OUTBOUND_PG_DB_DATA_TYPE_UINT64 8
  #define OUTBOUND_PG_DB_DATA_TYPE_FLOATING32 9
  #define OUTBOUND_PG_DB_DATA_TYPE_FLOATING64 10
  #define OUTBOUND_PG_DB_DATA_TYPE_STR 11
  #define OUTBOUND_PG_DB_DATA_TYPE_BINARY 12
  #define OUTBOUND_PG_DB_DATA_TYPE_OTHER 13
  typedef struct {
    outbound_pg_string_t name;
    outbound_pg_db_data_type_t data_type;
  } outbound_pg_column_t;
  void outbound_pg_column_free(outbound_pg_column_t *ptr);
  typedef struct {
    uint8_t *ptr;
    size_t len;
  } outbound_pg_list_u8_t;
  void outbound_pg_list_u8_free(outbound_pg_list_u8_t *ptr);
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
      outbound_pg_string_t str;
      outbound_pg_list_u8_t binary;
    } val;
  } outbound_pg_db_value_t;
  #define OUTBOUND_PG_DB_VALUE_BOOLEAN 0
  #define OUTBOUND_PG_DB_VALUE_INT8 1
  #define OUTBOUND_PG_DB_VALUE_INT16 2
  #define OUTBOUND_PG_DB_VALUE_INT32 3
  #define OUTBOUND_PG_DB_VALUE_INT64 4
  #define OUTBOUND_PG_DB_VALUE_UINT8 5
  #define OUTBOUND_PG_DB_VALUE_UINT16 6
  #define OUTBOUND_PG_DB_VALUE_UINT32 7
  #define OUTBOUND_PG_DB_VALUE_UINT64 8
  #define OUTBOUND_PG_DB_VALUE_FLOATING32 9
  #define OUTBOUND_PG_DB_VALUE_FLOATING64 10
  #define OUTBOUND_PG_DB_VALUE_STR 11
  #define OUTBOUND_PG_DB_VALUE_BINARY 12
  #define OUTBOUND_PG_DB_VALUE_DB_NULL 13
  #define OUTBOUND_PG_DB_VALUE_UNSUPPORTED 14
  void outbound_pg_db_value_free(outbound_pg_db_value_t *ptr);
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
      outbound_pg_string_t str;
      outbound_pg_list_u8_t binary;
    } val;
  } outbound_pg_parameter_value_t;
  #define OUTBOUND_PG_PARAMETER_VALUE_BOOLEAN 0
  #define OUTBOUND_PG_PARAMETER_VALUE_INT8 1
  #define OUTBOUND_PG_PARAMETER_VALUE_INT16 2
  #define OUTBOUND_PG_PARAMETER_VALUE_INT32 3
  #define OUTBOUND_PG_PARAMETER_VALUE_INT64 4
  #define OUTBOUND_PG_PARAMETER_VALUE_UINT8 5
  #define OUTBOUND_PG_PARAMETER_VALUE_UINT16 6
  #define OUTBOUND_PG_PARAMETER_VALUE_UINT32 7
  #define OUTBOUND_PG_PARAMETER_VALUE_UINT64 8
  #define OUTBOUND_PG_PARAMETER_VALUE_FLOATING32 9
  #define OUTBOUND_PG_PARAMETER_VALUE_FLOATING64 10
  #define OUTBOUND_PG_PARAMETER_VALUE_STR 11
  #define OUTBOUND_PG_PARAMETER_VALUE_BINARY 12
  #define OUTBOUND_PG_PARAMETER_VALUE_DB_NULL 13
  void outbound_pg_parameter_value_free(outbound_pg_parameter_value_t *ptr);
  typedef struct {
    outbound_pg_db_value_t *ptr;
    size_t len;
  } outbound_pg_row_t;
  void outbound_pg_row_free(outbound_pg_row_t *ptr);
  typedef struct {
    outbound_pg_column_t *ptr;
    size_t len;
  } outbound_pg_list_column_t;
  void outbound_pg_list_column_free(outbound_pg_list_column_t *ptr);
  typedef struct {
    outbound_pg_row_t *ptr;
    size_t len;
  } outbound_pg_list_row_t;
  void outbound_pg_list_row_free(outbound_pg_list_row_t *ptr);
  typedef struct {
    outbound_pg_list_column_t columns;
    outbound_pg_list_row_t rows;
  } outbound_pg_row_set_t;
  void outbound_pg_row_set_free(outbound_pg_row_set_t *ptr);
  typedef struct {
    outbound_pg_parameter_value_t *ptr;
    size_t len;
  } outbound_pg_list_parameter_value_t;
  void outbound_pg_list_parameter_value_free(outbound_pg_list_parameter_value_t *ptr);
  typedef struct {
    bool is_err;
    union {
      outbound_pg_row_set_t ok;
      outbound_pg_pg_error_t err;
    } val;
  } outbound_pg_expected_row_set_pg_error_t;
  void outbound_pg_expected_row_set_pg_error_free(outbound_pg_expected_row_set_pg_error_t *ptr);
  typedef struct {
    bool is_err;
    union {
      uint64_t ok;
      outbound_pg_pg_error_t err;
    } val;
  } outbound_pg_expected_u64_pg_error_t;
  void outbound_pg_expected_u64_pg_error_free(outbound_pg_expected_u64_pg_error_t *ptr);
  void outbound_pg_query(outbound_pg_string_t *address, outbound_pg_string_t *statement, outbound_pg_list_parameter_value_t *params, outbound_pg_expected_row_set_pg_error_t *ret0);
  void outbound_pg_execute(outbound_pg_string_t *address, outbound_pg_string_t *statement, outbound_pg_list_parameter_value_t *params, outbound_pg_expected_u64_pg_error_t *ret0);
  #ifdef __cplusplus
}
#endif
#endif

#ifndef __BINDINGS_LLM_H
#define __BINDINGS_LLM_H
#ifdef __cplusplus
extern "C"
{
  #endif
  
  #include <stdint.h>
  #include <stdbool.h>
  
  typedef struct {
    char *ptr;
    size_t len;
  } llm_string_t;
  
  void llm_string_set(llm_string_t *ret, const char *s);
  void llm_string_dup(llm_string_t *ret, const char *s);
  void llm_string_free(llm_string_t *ret);
  // A Large Language Model.
  typedef llm_string_t llm_inferencing_model_t;
  void llm_inferencing_model_free(llm_inferencing_model_t *ptr);
  // Inference request parameters
  typedef struct {
    uint32_t max_tokens;
    float repeat_penalty;
    uint32_t repeat_penalty_last_n_token_count;
    float temperature;
    uint32_t top_k;
    float top_p;
  } llm_inferencing_params_t;
  // The set of errors which may be raised by functions in this interface
  typedef struct {
    uint8_t tag;
    union {
      llm_string_t runtime_error;
      llm_string_t invalid_input;
    } val;
  } llm_error_t;
  #define LLM_ERROR_MODEL_NOT_SUPPORTED 0
  #define LLM_ERROR_RUNTIME_ERROR 1
  #define LLM_ERROR_INVALID_INPUT 2
  void llm_error_free(llm_error_t *ptr);
  // Usage information related to the inferencing result
  typedef struct {
    uint32_t prompt_token_count;
    uint32_t generated_token_count;
  } llm_inferencing_usage_t;
  // An inferencing result
  typedef struct {
    llm_string_t text;
    llm_inferencing_usage_t usage;
  } llm_inferencing_result_t;
  void llm_inferencing_result_free(llm_inferencing_result_t *ptr);
  // The model used for generating embeddings
  typedef llm_string_t llm_embedding_model_t;
  void llm_embedding_model_free(llm_embedding_model_t *ptr);
  typedef struct {
    float *ptr;
    size_t len;
  } llm_list_float32_t;
  void llm_list_float32_free(llm_list_float32_t *ptr);
  typedef struct {
    llm_list_float32_t *ptr;
    size_t len;
  } llm_list_list_float32_t;
  void llm_list_list_float32_free(llm_list_list_float32_t *ptr);
  // Usage related to an embeddings generation request
  typedef struct {
    uint32_t prompt_token_count;
  } llm_embeddings_usage_t;
  // Result of generating embeddings
  typedef struct {
    llm_list_list_float32_t embeddings;
    llm_embeddings_usage_t usage;
  } llm_embeddings_result_t;
  void llm_embeddings_result_free(llm_embeddings_result_t *ptr);
  typedef struct {
    bool is_some;
    llm_inferencing_params_t val;
  } llm_option_inferencing_params_t;
  typedef struct {
    bool is_err;
    union {
      llm_inferencing_result_t ok;
      llm_error_t err;
    } val;
  } llm_expected_inferencing_result_error_t;
  void llm_expected_inferencing_result_error_free(llm_expected_inferencing_result_error_t *ptr);
  typedef struct {
    llm_string_t *ptr;
    size_t len;
  } llm_list_string_t;
  void llm_list_string_free(llm_list_string_t *ptr);
  typedef struct {
    bool is_err;
    union {
      llm_embeddings_result_t ok;
      llm_error_t err;
    } val;
  } llm_expected_embeddings_result_error_t;
  void llm_expected_embeddings_result_error_free(llm_expected_embeddings_result_error_t *ptr);
  void llm_infer(llm_inferencing_model_t *model, llm_string_t *prompt, llm_option_inferencing_params_t *params, llm_expected_inferencing_result_error_t *ret0);
  void llm_generate_embeddings(llm_embedding_model_t *model, llm_list_string_t *text, llm_expected_embeddings_result_error_t *ret0);
  #ifdef __cplusplus
}
#endif
#endif

#include <stdlib.h>
#include <llm.h>

__attribute__((weak, export_name("canonical_abi_realloc")))
void *canonical_abi_realloc(
void *ptr,
size_t orig_size,
size_t align,
size_t new_size
) {
  if (new_size == 0)
  return (void*) align;
  void *ret = realloc(ptr, new_size);
  if (!ret)
  abort();
  return ret;
}

__attribute__((weak, export_name("canonical_abi_free")))
void canonical_abi_free(
void *ptr,
size_t size,
size_t align
) {
  if (size == 0)
  return;
  free(ptr);
}
#include <string.h>

void llm_string_set(llm_string_t *ret, const char *s) {
  ret->ptr = (char*) s;
  ret->len = strlen(s);
}

void llm_string_dup(llm_string_t *ret, const char *s) {
  ret->len = strlen(s);
  ret->ptr = canonical_abi_realloc(NULL, 0, 1, ret->len);
  memcpy(ret->ptr, s, ret->len);
}

void llm_string_free(llm_string_t *ret) {
  canonical_abi_free(ret->ptr, ret->len, 1);
  ret->ptr = NULL;
  ret->len = 0;
}
void llm_inferencing_model_free(llm_inferencing_model_t *ptr) {
  llm_string_free(ptr);
}
void llm_error_free(llm_error_t *ptr) {
  switch ((int32_t) ptr->tag) {
    case 1: {
      llm_string_free(&ptr->val.runtime_error);
      break;
    }
    case 2: {
      llm_string_free(&ptr->val.invalid_input);
      break;
    }
  }
}
void llm_inferencing_result_free(llm_inferencing_result_t *ptr) {
  llm_string_free(&ptr->text);
}
void llm_embedding_model_free(llm_embedding_model_t *ptr) {
  llm_string_free(ptr);
}
void llm_list_float32_free(llm_list_float32_t *ptr) {
  canonical_abi_free(ptr->ptr, ptr->len * 4, 4);
}
void llm_list_list_float32_free(llm_list_list_float32_t *ptr) {
  for (size_t i = 0; i < ptr->len; i++) {
    llm_list_float32_free(&ptr->ptr[i]);
  }
  canonical_abi_free(ptr->ptr, ptr->len * 8, 4);
}
void llm_embeddings_result_free(llm_embeddings_result_t *ptr) {
  llm_list_list_float32_free(&ptr->embeddings);
}
void llm_expected_inferencing_result_error_free(llm_expected_inferencing_result_error_t *ptr) {
  if (!ptr->is_err) {
    llm_inferencing_result_free(&ptr->val.ok);
  } else {
    llm_error_free(&ptr->val.err);
  }
}
void llm_list_string_free(llm_list_string_t *ptr) {
  for (size_t i = 0; i < ptr->len; i++) {
    llm_string_free(&ptr->ptr[i]);
  }
  canonical_abi_free(ptr->ptr, ptr->len * 8, 4);
}
void llm_expected_embeddings_result_error_free(llm_expected_embeddings_result_error_t *ptr) {
  if (!ptr->is_err) {
    llm_embeddings_result_free(&ptr->val.ok);
  } else {
    llm_error_free(&ptr->val.err);
  }
}

__attribute__((aligned(4)))
static uint8_t RET_AREA[20];
__attribute__((import_module("llm"), import_name("infer")))
void __wasm_import_llm_infer(int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, float, int32_t, float, int32_t, float, int32_t);
void llm_infer(llm_inferencing_model_t *model, llm_string_t *prompt, llm_option_inferencing_params_t *params, llm_expected_inferencing_result_error_t *ret0) {
  int32_t option;
  int32_t option1;
  float option2;
  int32_t option3;
  float option4;
  int32_t option5;
  float option6;
  
  if ((*params).is_some) {
    const llm_inferencing_params_t *payload0 = &(*params).val;
    option = 1;
    option1 = (int32_t) ((*payload0).max_tokens);
    option2 = (*payload0).repeat_penalty;
    option3 = (int32_t) ((*payload0).repeat_penalty_last_n_token_count);
    option4 = (*payload0).temperature;
    option5 = (int32_t) ((*payload0).top_k);
    option6 = (*payload0).top_p;
    
  } else {
    option = 0;
    option1 = 0;
    option2 = 0;
    option3 = 0;
    option4 = 0;
    option5 = 0;
    option6 = 0;
    
  }
  int32_t ptr = (int32_t) &RET_AREA;
  __wasm_import_llm_infer((int32_t) (*model).ptr, (int32_t) (*model).len, (int32_t) (*prompt).ptr, (int32_t) (*prompt).len, option, option1, option2, option3, option4, option5, option6, ptr);
  llm_expected_inferencing_result_error_t expected;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      expected.is_err = false;
      
      expected.val.ok = (llm_inferencing_result_t) {
        (llm_string_t) { (char*)(*((int32_t*) (ptr + 4))), (size_t)(*((int32_t*) (ptr + 8))) },
        (llm_inferencing_usage_t) {
          (uint32_t) (*((int32_t*) (ptr + 12))),
          (uint32_t) (*((int32_t*) (ptr + 16))),
        },
      };
      break;
    }
    case 1: {
      expected.is_err = true;
      llm_error_t variant;
      variant.tag = (int32_t) (*((uint8_t*) (ptr + 4)));
      switch ((int32_t) variant.tag) {
        case 0: {
          break;
        }
        case 1: {
          variant.val.runtime_error = (llm_string_t) { (char*)(*((int32_t*) (ptr + 8))), (size_t)(*((int32_t*) (ptr + 12))) };
          break;
        }
        case 2: {
          variant.val.invalid_input = (llm_string_t) { (char*)(*((int32_t*) (ptr + 8))), (size_t)(*((int32_t*) (ptr + 12))) };
          break;
        }
      }
      
      expected.val.err = variant;
      break;
    }
  }*ret0 = expected;
}
__attribute__((import_module("llm"), import_name("generate-embeddings")))
void __wasm_import_llm_generate_embeddings(int32_t, int32_t, int32_t, int32_t, int32_t);
void llm_generate_embeddings(llm_embedding_model_t *model, llm_list_string_t *text, llm_expected_embeddings_result_error_t *ret0) {
  int32_t ptr = (int32_t) &RET_AREA;
  __wasm_import_llm_generate_embeddings((int32_t) (*model).ptr, (int32_t) (*model).len, (int32_t) (*text).ptr, (int32_t) (*text).len, ptr);
  llm_expected_embeddings_result_error_t expected;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      expected.is_err = false;
      
      expected.val.ok = (llm_embeddings_result_t) {
        (llm_list_list_float32_t) { (llm_list_float32_t*)(*((int32_t*) (ptr + 4))), (size_t)(*((int32_t*) (ptr + 8))) },
        (llm_embeddings_usage_t) {
          (uint32_t) (*((int32_t*) (ptr + 12))),
        },
      };
      break;
    }
    case 1: {
      expected.is_err = true;
      llm_error_t variant;
      variant.tag = (int32_t) (*((uint8_t*) (ptr + 4)));
      switch ((int32_t) variant.tag) {
        case 0: {
          break;
        }
        case 1: {
          variant.val.runtime_error = (llm_string_t) { (char*)(*((int32_t*) (ptr + 8))), (size_t)(*((int32_t*) (ptr + 12))) };
          break;
        }
        case 2: {
          variant.val.invalid_input = (llm_string_t) { (char*)(*((int32_t*) (ptr + 8))), (size_t)(*((int32_t*) (ptr + 12))) };
          break;
        }
      }
      
      expected.val.err = variant;
      break;
    }
  }*ret0 = expected;
}

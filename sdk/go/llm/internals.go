package llm

// #include "llm.h"
import "C"
import (
	"errors"
	"fmt"
	"unsafe"
)

func infer(model, prompt string, params *InferencingParams) (*InferencingResult, error) {
	llmModel := toLLMModel(model)
	llmPrompt := toLLMString(prompt)
	llmParams := toLLMInferencingParams(params)

	var ret C.llm_expected_inferencing_result_error_t
	defer C.llm_expected_inferencing_result_error_free(&ret)

	C.llm_infer(&llmModel, &llmPrompt, &llmParams, &ret)
	if ret.is_err {
		return nil, toErr((*C.llm_error_t)(unsafe.Pointer(&ret.val)))
	}

	result := (*C.llm_inferencing_result_t)(unsafe.Pointer(&ret.val))

	r := &InferencingResult{
		Text: C.GoStringN(result.text.ptr, C.int(result.text.len)),
		Usage: &InferencingUsage{
			PromptTokenCount:    int(result.usage.prompt_token_count),
			GeneratedTokenCount: int(result.usage.generated_token_count),
		},
	}
	return r, nil
}

func toErr(err *C.llm_error_t) error {
	switch err.tag {
	case 0:
		return errors.New("model not supported")
	case 1:
		str := (*C.llm_string_t)(unsafe.Pointer(&err.val))
		return fmt.Errorf("runtime error: %s", C.GoStringN(str.ptr, C.int(str.len)))
	case 2:
		str := (*C.llm_string_t)(unsafe.Pointer(&err.val))
		return fmt.Errorf("invalid input error: %s", C.GoStringN(str.ptr, C.int(str.len)))
	default:
		return fmt.Errorf("unrecognized error: %v", err.tag)
	}
}

func toLLMModel(name string) C.llm_inferencing_model_t {
	llmString := toLLMString(name)
	return *(*C.llm_inferencing_model_t)(unsafe.Pointer(&llmString.ptr))
}

func toLLMString(x string) C.llm_string_t {
	return C.llm_string_t{ptr: C.CString(x), len: C.size_t(len(x))}
}

func toLLMInferencingParams(p *InferencingParams) C.llm_option_inferencing_params_t {
	if p == nil {
		return C.llm_option_inferencing_params_t{is_some: false}
	}
	llmParams := C.llm_inferencing_params_t{
		max_tokens:                        C.uint32_t(p.MaxTokens),
		repeat_penalty:                    C.float(p.RepeatPenalty),
		repeat_penalty_last_n_token_count: C.uint32_t(p.RepeatPenaltyLastNTokenCount),
		temperature:                       C.float(p.Temperature),
		top_k:                             C.uint32_t(p.TopK),
		top_p:                             C.float(p.TopP),
	}
	return C.llm_option_inferencing_params_t{is_some: true, val: llmParams}
}

func generateEmbeddings(model string, text []string) (*EmbeddingsResult, error) {
	llmModel := toLLMEmbeddingModel(model)
	llmListString := toLLMListString(text)

	var ret C.llm_expected_embeddings_result_error_t
	defer C.llm_expected_embeddings_result_error_free(&ret)

	C.llm_generate_embeddings(&llmModel, &llmListString, &ret)
	if ret.is_err {
		return nil, toErr((*C.llm_error_t)(unsafe.Pointer(&ret.val)))
	}

	result := (*C.llm_embeddings_result_t)(unsafe.Pointer(&ret.val))

	r := &EmbeddingsResult{
		Embeddings: fromLLMListListFloat32(result.embeddings),
		Usage: &EmbeddingsUsage{
			PromptTokenCount: int(result.usage.prompt_token_count),
		},
	}
	return r, nil
}

func toLLMEmbeddingModel(name string) C.llm_embedding_model_t {
	llmString := toLLMString(name)
	return *(*C.llm_embedding_model_t)(unsafe.Pointer(&llmString.ptr))
}

func toLLMListString(xs []string) C.llm_list_string_t {
	cxs := make([]C.llm_string_t, len(xs))
	for i := 0; i < len(xs); i++ {
		cxs[i] = toLLMString(xs[i])
	}
	return C.llm_list_string_t{ptr: &cxs[0], len: C.size_t(len(cxs))}
}

func fromLLMListListFloat32(list C.llm_list_list_float32_t) [][]float32 {
	listLen := int(list.len)
	ret := make([][]float32, listLen)
	slice := unsafe.Slice(list.ptr, listLen)
	for i := 0; i < listLen; i++ {
		row := *((*C.llm_list_float32_t)(unsafe.Pointer(&slice[i])))
		ret[i] = fromLLMListFloat32(row)
	}
	return ret
}

func fromLLMListFloat32(list C.llm_list_float32_t) []float32 {
	listLen := int(list.len)
	ret := make([]float32, listLen)
	slice := unsafe.Slice(list.ptr, listLen)
	for i := 0; i < listLen; i++ {
		v := *((*C.float)(unsafe.Pointer(&slice[i])))
		ret[i] = float32(v)
	}
	return ret
}

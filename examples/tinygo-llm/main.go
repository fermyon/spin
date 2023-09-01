package main

import (
	"fmt"
	"net/http"

	spinhttp "github.com/fermyon/spin/sdk/go/http"
	"github.com/fermyon/spin/sdk/go/llm"
)

func init() {
	spinhttp.Handle(func(w http.ResponseWriter, r *http.Request) {
		params := &llm.InferencingParams{
			MaxTokens:                     100,
			RepeatPenality:                1.1,
			RepeatPenalityLastNTokenCount: 64,
			Temperature:                   0.8,
			TopK:                          40,
			TopP:                          0.9,
		}

		result, err := llm.Infer("llama2-chat", "Is Radu sleeping?", params)
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		fmt.Printf("Prompt tokens:    %d\n", result.Usage.PromptTokenCount)
		fmt.Printf("Generated tokens: %d\n", result.Usage.GeneratedTokenCount)
		fmt.Fprint(w, result.Text)
		fmt.Fprintf(w, "\n\n")

		embeddings, err := llm.GenerateEmbeddings("all-minilm-l6-v2", []string{"Hello world"})
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		fmt.Printf("%d\n", len(embeddings.Embeddings[0]))
		fmt.Printf("Prompt Tokens: %d\n", embeddings.Usage.PromptTokenCount)

	})
}

func main() {}

package main

import (
	"fmt"
	"net/http"

	spinhttp "github.com/fermyon/spin/sdk/go/v2/http"
	"github.com/fermyon/spin/sdk/go/v2/llm"
)

func init() {
	spinhttp.Handle(func(w http.ResponseWriter, r *http.Request) {
		result, err := llm.Infer("llama2-chat", "Tell me a joke", nil)
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

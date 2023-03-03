package main

import (
	"fmt"
	"net/http"
	"os"

	spin_http "github.com/fermyon/spin/sdk/go/http"
	"github.com/fermyon/spin/sdk/go/postgres"
)

func init() {

	// handler for the http trigger
	spin_http.Handle(func(w http.ResponseWriter, r *http.Request) {

		addr := os.Getenv("DB_URL")

		if err := write(addr, w, r); err != nil {
			//TODO: log the error?
			return
		}

		if err := read(addr, w, r); err != nil {
			//TODO: log the error?
			return
		}
	})
}

func main() {}

type Article struct {
	id         int32
	title      string
	content    string
	authorname string
	coauthor   string
}

func (a Article) Json() string {
	//TODO: use a json serializer
	fmt_str := "{" +
		`"id": "%v",` +
		`"title": "%v",` +
		`"content": "%v",` +
		`"authorname": "%v",` +
		`"coauthor": "%v"` +
		"}"
	return fmt.Sprintf(fmt_str, a.id, a.title, a.content, a.authorname, a.coauthor)
}

func articleFromRow(row []postgres.DbValue) (Article, error) {
	var article Article
	article.id = row[0].GetInt32()
	article.title = row[1].GetStr()
	article.content = row[2].GetStr()
	article.authorname = row[3].GetStr()
	if row[4].Kind() != postgres.DbValueKindDbNull {
		article.coauthor = row[4].GetStr()
	}
	return article, nil
}

func read(addr string, w http.ResponseWriter, r *http.Request) error {
	fmt.Println("Querying from articletest")
	statement := "SELECT id, title, content, authorname, coauthor FROM articletest"
	rowset, err := postgres.Query(addr, statement, []postgres.ParameterValue{})
	if err != nil {
		return err
	}

	fmt.Println("Summarizing the columns")
	col_count := len(rowset.Columns)
	fmt.Fprintf(w, "Columns = ")
	for i := 0; i < col_count; i++ {
		col := rowset.Columns[i]
		fmt.Fprintf(w, "%v, ", col.Name)
	}
	fmt.Fprintln(w)

	fmt.Println("Writing all rows to response")
	for _, row := range rowset.Rows {
		article, err := articleFromRow(row)
		if err != nil {
			//TODO: write the error to response
			return err
		}
		fmt.Fprint(w, article.Json())
	}

	return nil
}

func write(addr string, w http.ResponseWriter, r *http.Request) error {
	fmt.Println("Inserting into articletest")

	title := "Test Article"
	content := "This article was inserted by the example module"
	authorname := "spin"
	coauthor := "tingyo-outbound-pg"
	statement := "INSERT INTO articletest (title, content, authorname, coauthor) VALUES ($1, $2, $3, $4)"
	params := []postgres.ParameterValue{
		postgres.ParameterValueStr(title),
		postgres.ParameterValueStr(content),
		postgres.ParameterValueStr(authorname),
		postgres.ParameterValueStr(coauthor),
	}
	n, err := postgres.Execute(addr, statement, params)
	fmt.Fprintf(w, "Inserted rows=%v\n", n)
	return err
}

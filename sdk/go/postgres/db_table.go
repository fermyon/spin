package postgres

type Column struct {
	Name     string
	DataType DbDataType
}

type RowSet struct {
	Columns []Column
	Rows    [][]DbValue
}

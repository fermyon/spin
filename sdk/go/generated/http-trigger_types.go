package http_trigger

// inspired from https://github.com/moznion/go-optional

type optionKind int

const (
none optionKind = iota
some
)

type Option[T any] struct {
  kind optionKind
  val  T
}

// IsNone returns true if the option is None.
func (o Option[T]) IsNone() bool {
  return o.kind == none
}

// IsSome returns true if the option is Some.
func (o Option[T]) IsSome() bool {
  return o.kind == some
}

// Unwrap returns the value if the option is Some.
func (o Option[T]) Unwrap() T {
  if o.kind != some {
    panic("Option is None")
  }
  return o.val
}

// Set sets the value and returns it.
func (o *Option[T]) Set(val T) T {
  o.kind = some
  o.val = val
  return val
}

// Unset sets the value to None.
func (o *Option[T]) Unset() {
  o.kind = none
}

// Some is a constructor for Option[T] which represents Some.
func Some[T any](v T) Option[T] {
  return Option[T]{
    kind: some,
    val:  v,
  }
}

// None is a constructor for Option[T] which represents None.
func None[T any]() Option[T] {
  return Option[T]{
    kind: none,
  }
}

type ResultKind int

const (
Ok ResultKind = iota
Err
)

type Result[T any, E any] struct {
  Kind ResultKind
  Val  T
  Err  E
}

func (r Result[T, E]) IsOk() bool {
  return r.Kind == Ok
}

func (r Result[T, E]) IsErr() bool {
  return r.Kind == Err
}

func (r Result[T, E]) Unwrap() T {
  if r.Kind != Ok {
    panic("Result is Err")
  }
  return r.Val
}

func (r Result[T, E]) UnwrapErr() E {
  if r.Kind != Err {
    panic("Result is Ok")
  }
  return r.Err
}

func (r *Result[T, E]) Set(val T) T {
  r.Kind = Ok
  r.Val = val
  return val
}

func (r *Result[T, E]) SetErr(err E) E {
  r.Kind = Err
  r.Err = err
  return err
}


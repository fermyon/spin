Contains apps used to test `spin build`.

# `component` build command
It should be quick, not produce large artifacts and work cross-platform without prerequisites:

```
# Create a file which is _almost_ empty (its content is ignored).
echo '' > foo
```

Then, it can be checked that `foo` exists to verify that `spin build` succeeded.

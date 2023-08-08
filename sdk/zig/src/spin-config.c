#include <stdlib.h>

__attribute__((import_module("spin-config"), import_name("get-config"))) void
    __wasm_import_spin_config_get_config(int32_t, int32_t, int32_t);

__attribute__((weak, export_name("canonical_abi_realloc"))) void*
canonical_abi_realloc(void* ptr,
    size_t orig_size,
    size_t align,
    size_t new_size)
{
    if (new_size == 0) {
        return (void*)align;
    }

    void* ret = realloc(ptr, new_size);

    if (!ret) {
        abort();
    }
    return ret;
}

__attribute__((weak, export_name("canonical_abi_free"))) void
canonical_abi_free(void* ptr, size_t size, size_t align)
{
    if (size == 0) {
        return;
    }

    free(ptr);
}

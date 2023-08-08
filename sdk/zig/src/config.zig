const std = @import("std");

pub const Error = union(enum) {
    invalid_schema: []u8,
    invalid_key: []u8,
    provider: []u8,
    other: []u8,
};

pub const Result = union(enum) {
    err: Error,
    ok: []u8,
};

extern "spin-config" fn __wasm_import_get_config(usize, usize, usize) void;

pub fn get(key: []const u8) Result {
    var res_arr = [1]usize{0} ** 4;

    __wasm_import_get_config(@intFromPtr(key.ptr), key.len, @intFromPtr(&res_arr));

    var res: Result = undefined;
    var str: []u8 = undefined;

    switch (res_arr[0]) {
        0 => {
            str.ptr = @ptrFromInt(res_arr[1]);
            str.len = res_arr[2];
            res = .{ .ok = str };
        },
        1 => {
            str.ptr = @ptrFromInt(res_arr[2]);
            str.len = res_arr[3];
            switch (res_arr[1]) {
                0 => res = .{ .err = .{ .provider = str } },
                1 => res = .{ .err = .{ .invalid_key = str } },
                2 => res = .{ .err = .{ .invalid_schema = str } },
                3 => res = .{ .err = .{ .other = str } },
                else => unreachable,
            }
        },
        else => unreachable,
    }

    return res;
}

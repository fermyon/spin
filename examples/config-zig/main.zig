const std = @import("std");
const config = @import("spin").config;

const log = std.log.scoped(.config);

const Error = std.os.WriteError;

pub fn main() Error!void {
    const std_out = std.io.getStdOut();
    var buf_writer = std.io.bufferedWriter(std_out.writer());
    const writer = buf_writer.writer();

    try writer.writeAll("content-type: text/plain\n\n");

    const res = config.get("message");

    switch (res) {
        .ok => |str| try writer.print("message: {s}\n", .{str}),
        .err => |err| switch (err) {
            .invalid_schema => log.err("Invalid schema: {s}", .{err.invalid_schema}),
            .invalid_key => log.err("Invalid key: {s}", .{err.invalid_key}),
            .provider => log.err("Provider: {s}", .{err.provider}),
            .other => log.err("Other: {s}", .{err.other}),
        },
    }

    try buf_writer.flush();
}

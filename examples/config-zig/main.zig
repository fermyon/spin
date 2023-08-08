const std = @import("std");
const config = @import("spin").config;

const Error = error{
    HttpStatusInternalServerError,
} || std.os.WriteError;

pub fn main() Error!void {
    const std_out = std.io.getStdOut();
    var buf_writer = std.io.bufferedWriter(std_out.writer());
    const writer = buf_writer.writer();

    try writer.writeAll("content-type: text/plain\n\n");

    const res = config.get("message");

    switch (res) {
        .ok => |str| try writer.print("message: {s}\n", .{str}),
        .err => return error.HttpStatusInternalServerError,
    }

    try buf_writer.flush();
}

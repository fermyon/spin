const std = @import("std");

const Error = std.os.WriteError || std.fs.File.OpenError;

pub fn build(b: *std.Build) Error!void {
    // Set SDK build metadata
    var sdk_file = try std.fs.cwd().createFile(SRC_DIR ++ "sdk-version-zig.c", .{});
    var buf_writer = std.io.bufferedWriter(sdk_file.writer());
    const writer = buf_writer.writer();

    const sdk_template = @embedFile(SRC_DIR ++ "sdk-version-zig-template.c");
    const version = b.exec(&.{ "cargo", "run", "--manifest-path", "sdk/rust/Cargo.toml" });
    const commit = b.exec(&.{ "git", "rev-parse", "HEAD" })[0..40];

    try writer.writeAll(sdk_template[0..51]);
    try writer.writeAll(version);
    try writer.writeAll(sdk_template[51..141]);
    try writer.writeAll(commit);
    try writer.writeAll(sdk_template[141..]);

    try buf_writer.flush();

    // Module
    const spin_mod = b.addModule("spin", .{ .source_file = .{ .path = SRC_DIR ++ "spin.zig" } });

    // Binding generation
    const bindgen_step = b.step("bindgen", "Generate WIT bindings for C guest modules");

    inline for (BINDGEN_FILES) |BINDGEN_FILE| {
        const bindgen_run = b.addSystemCommand(&.{ "wit-bindgen", "c", "--import", BINDGEN_FILE, "--out-dir", SRC_DIR });

        bindgen_step.dependOn(&bindgen_run.step);
    }

    b.default_step.dependOn(bindgen_step);

    // Examples
    const examples_step = b.step("example", "Install examples");

    inline for (EXAMPLE_NAMES) |EXAMPLE_NAME| {
        const example = b.addExecutable(.{
            .name = EXAMPLE_NAME,
            .root_source_file = std.Build.FileSource.relative(EXAMPLES_DIR ++ EXAMPLE_NAME ++ "-zig/main.zig"),
            .target = .{ .cpu_arch = .wasm32, .os_tag = .wasi },
            .optimize = .ReleaseSmall,
        });
        example.addModule("spin", spin_mod);

        const example_install = b.addInstallArtifact(example, .{});

        examples_step.dependOn(&example_install.step);
    }

    b.default_step.dependOn(examples_step);

    // Lints
    const lints_step = b.step("lint", "Run lints");

    const lints = b.addFmt(.{
        .paths = &.{ EXAMPLES_DIR, SRC_DIR, "build.zig" },
        .check = true,
    });

    lints_step.dependOn(&lints.step);
    b.default_step.dependOn(lints_step);
}

const SRC_DIR = "sdk/zig/src/";

const BINDGEN_DIR = "wit/ephemeral/";

const BINDGEN_FILES = &.{
    BINDGEN_DIR ++ "spin-config.wit",
};

const EXAMPLES_DIR = "examples/";

const EXAMPLE_NAMES = &.{
    "config",
};

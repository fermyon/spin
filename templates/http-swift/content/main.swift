import WASILibc

// Until all of ProcessInfo makes its way into SwiftWasm
func getEnvVar(key: String) -> Optional<String> {
    guard let rawValue = getenv(key) else {return Optional.none}
    return String(validatingUTF8: rawValue)
}

let server = getEnvVar(key: "SERVER_SOFTWARE") ?? "Unknown Server"
let message = """
content-type: text/plain

Hello from \(server)!
"""

print(message)

title = "SIP 018 - Application Metadata"
template = "main"
date = "2024-04-12T23:00:00Z"

---

Summary: Allow guest code to access information from the application manifest

Owner(s): ivan.towlson@fermyon.com

Created: Apr 12, 2024

# Requirements and scope

We have, over time, had a few requests for guest code to be able to read fields from the application manifest, or from the host itself. Examples include:

| Field                                         | Purpose          | Notes |
|-----------------------------------------------|------------------|-------|
| Application name and version                  | Logging          |       |
| Host runtime name or other identifying string | Logging          |       |
| Host runtime version                          | Logging          |       |
| Component id                                  | Logging (?)      |       |
| Files mounted to the component                | Not given        | Already available by traversing the component's file system |
| Allowed outbound hosts for the component      | Not given        | One user mentioned their app needed to scan a set of sites but I don't have details |
| Allowed KV/SQLite stores for the component    | Explorer UI      |       |
| Component HTTP route                          | Not given        | Already available via the `spin-raw-component-route` header |
| The component's build directory               | Guest files mapped at `workdir` | Not clear why user mapped to the subdirectory |

## Should we?

We've been a bit wary of doing some of this stuff because the Wasm model is that components should define the set of capabilities they consume, and be granted only those capabilities, so there should be no need for them to be going around sniffing to find out what goodies they have access to. This concern applies specifically to the "allowed" lists (outbound hosts and KV/SQLite). Possibly this would be addressed by requiring components to declare access.

Also, I'd argue we should _not_ provide access to `workdir`. This is, I believe, not currently available at runtime; and in any case it should have nothing to do with what happens at runtime - build is way distant in the rear view mirror by then. This is better handled by mapping files up a level or by passing a distinct "file root" variable.

| Field                                         | Recommendation   |
|-----------------------------------------------|------------------|
| Application name and version                  | Unproblematic    |
| Host runtime name or other identifying string | Unproblematic    |
| Host runtime version                          | Unproblematic    |
| Component id                                  | Unproblematic    |
| Files mounted to the component                | Superfluous      |
| Allowed outbound hosts for the component      | Caution advised  |
| Allowed KV/SQLite stores for the component    | Caution advised  |
| Component HTTP route                          | Superfluous      |
| The component's build directory               | Should not       |

# User experience

## Access

If we do wish to expose this data, there are various ways we could do so:

1. Provide read access to the manifest file itself. This couples application logic to the manifest format du jour. We should not do this.

2. Create a new API that reads specific fields e.g. `application_meta::component_id()`. (A variant would be a single function accepting a field ID, e.g. `application_meta::get(AppMeta::ComponentId)`.) This is safe and discoverable, but does not line up with any broader conventions, and may have versioning implications if we need to add additional fields over time.

3. Provide environment variables containing the information e.g. `SPIN_APP_META_COMPONENT_ID`.

4. Wedge it into an existing API such as `variables` - e.g. have some special magic variable names that are implicitly mapped to these values. This limits us to strings, but provides a nice built-in permissions mechanism where a component has to indicate in its manifest what data it plans to exfiltrate.

## Permissions

We should decide which, if any, metadata is available by default.

Components should then have to declare any non-default metadata they use, in the same way as they declare variables or stores. The UI for this would look similar to those UIs: we can define that if we decide to do this.

# Runtime and framework considerations

Any runtime considerations will depend on the chosen user experience but seem unlikely to be vastly complicated.  Famous last words.

# Future possibilities

None.

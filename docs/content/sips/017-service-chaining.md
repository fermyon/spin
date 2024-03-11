title = "SIP 017 - Service Chaining"
template = "main"
date = "2024-02-21T23:00:00Z"

---

Summary: Allow in-process HTTP requests to application components without going via the network

Owner(s): ivan.towlson@fermyon.com

Created: Feb 21, 2024

# Requirements and scope

In Spin 2.2, a component can make a HTTP request with only a path (e.g. `/api/accounts`).  Such a request will be routed to the same application, but this is done by Spin prepending the host where the application is running (e.g. `https://mysrsbizness.fermyon.app/api/accounts`).  The consequence of this is that requests go out to the network and come back in, including through load balancers etc.  This adds overhead, and in some environnments could count against quota or incur bandwidth charges.

_Service chaining_ refers to carrying out HTTP requests in memory, without going off the node, by invoking the receiving Spin component directly instead of via the HTTP server.

We have [prototyped](https://github.com/fermyon/spin/pull/2229) dispatching HTTP requests directly to the component handling the route, but this was in the context of an implementation detail of Spin.  The requirement for this project is for an application to declare that it _requires_ dispatch to run locally on the node, and for hosts that cannot meet that prerequisite to reject the application.  This allows developers to write applications as a network of chained microservices, without having to be concerned about what happens if the microservices are not co-located.

# User experience

## Expressing a requirement for chaining

Spin will infer that an application requires service chaining if any component includes a `spin.internal` subdomain (or subdomain wildcard) in its `allowed_outbound_hosts`.

This has two implications:

1. The "allow all" wildcard (`*://*:*`) will not, by itself, enable service chaining.
2. A templated host (`https://{{ accounts_service }}:*`), with the variable set to a `spin.internal` subdomain, will not, by itself, allow service chaining.

Hosts must reject applications that ask for `spin.internal` if the host cannot provide service chaining. However, this will be achieved via the lockfile rather than via the `spin.toml`: see below for details.

## Requesting a component via chaining

There are two ways we could express a HTTP request to a chained component:

1. Use the existing self-request style.  The host _may_ chain self-requests, and _must_ chain them if the "require chaining" setting is on.  This is super easy for the application developer, at both the send and receive end.  However, it becomes more complex for the host, as the host now has to route the request - this drags in much more of the HTTP trigger plumbing.  (The prototype did this and it was egregiously hairy.)

2. Address requests directly to the destination components by name. We still need a URL, so this would introduce a special host name such as `spin.internal`.  Senders address individual components via subdomains.  Thus, the URL `http://accounts.spin.internal` would invoke the `accounts` component.  This bypasses routing, which should simplify implementation.  It also opens the door to private endpoints, where an internal microservice is not exposed to the network (does not have a HTTP route) and so is accessible only from within the application.  However, chained components which parse URLs may need to take extra care to correctly handle both routed and chaining URLs.

**The preferred user experience is option 2: `http://<component-name>.spin.internal/`**

## Incoming request headers

To avoid spoofing, Spin must strip any `Host: *.spin.internal` headers from routed requests.

Additionally, Spin will provide a `Host: <component>.spin.internal` header on the chained request, overwriting any host header set in the request.

Other Spin headers such as `spin-full-url` should be set where appropriate (e.g. `spin-path-info`) and omitted where not (e.g. `spin-client-addr`). The HTTP executor may handle this for us - this is a matter for implementation.

## Permissions

The developer must enable chaining to a component by adding `http://<component-name>.spin.internal`, or `http://*.spin.internal` to allow any destination component, to the list of allowed outbound hosts.

`self` permission does _not_ grant permissing to make chained requests; nor, as noted above, does the wildcard permission.

# Runtime and framework considerations

## Chaining requirements in the lock file

The lock file format built into existing hosts is designed for extension, and so [avoids using serde "deny unknown fields"](https://github.com/fermyon/spin/blob/663593d1423ed3518744d6f797e6a3970575d617/crates/locked-app/src/locked.rs#L16).  Unfortunately, this means there is no way to add a prerequisites section to the lock file in such a way that existing hosts will reject it if present.  For example, if someone deploys a chaining-required app onto an older version of the containerd shim, or even in the CLI to an external trigger built with pre-chaining crates, it will appear to accept it, but will not chain.

Unfortunately, I don't think there's anything we can do about this, except to introduce a new `spin_lock_version`.  Doing this naively would mean existing hosts would reject the lock file _whether or not it contains prerequisites_.  So we will define a v1 lockfile format that lists host requirements, but smartly emit lock file version 0 if the list of host requirements is empty.

To mitigate future cases like this, the v1 lockfile will add a `must_understand` section.  This is an array of strings describing features that the host is expected to understand/support: if it doesn't recognise a must-understand string, it must reject the application.

## Spin command line implementation

## Where do invoked components run?

In Spin, all components of an app run within a trigger process (typically a child Spin process, but can also be a plugin).

When the calling component is on the HTTP trigger, life is (relatively) easy, because the host process has all the HTTP components - that is, all possible invocation targets - loaded and ready to go.

When the calling command is on a _non-HTTP_ trigger, things are more challenging.  There are two options if we want to do this:

1. We must host a large chunk of the HTTP trigger engine (and load HTTP components) in the non-HTTP trigger process. Physically this means moving the HTTP execution engine into a crate that can be shared by the `outbound-http` and `trigger-http` crates. However, this will still need a lot of the same infrastructure that the full trigger does, e.g. loading host components, permissions, etc. - I don't think it will omit much except the hooking up to the server/router. The two different ways of loading HTTP components feels like an exciting area for innovating in subtle surprises, and this is without even thinking about external trigger processes embedding different versions of the HTTP engine...

2. We somehow transfer the request to the HTTP trigger for handling. This is both difficult _and_ unappealing. Let's not do it.

> _Additional context:_ Today, only HTTP components can make self-requests anyway (because non-HTTP components can't infer the host URL).

**For the initial release we will allow chaining only from HTTP components to HTTP components.**  This does not preclude allowing chaining from non-HTTP components in a future drop.

## Streaming and concurrency

It's possible for a caller to be streaming a request to a callee and the callee streaming its response at the same time.  The implementation needs to allow for this: it may just fall out for free from everything on the host side being async, but we need to test and make sure the bytes keep flowing!  Chained requests will not have hyper in the flow helpfully spawning tasks so we may end up needing to do it manually - this can be deferred to implementation.

## Failures

The implementation must determine and document what happens if the callee fails - specifically if it traps.  We expect that the trap will be caught by the HTTP infrastructure and returned to the caller as a 500 HTTP response, rather than the trap taking down the caller as well.  This behaviour is different from if the components were composed, but is acceptable.

# Other implementations

## Fermyon Cloud

Cloud must examine the lock file for must-understand prerequisites.  Ideally this would happen at deployment time (so that rejection could be returned via the API), but I understand that this may not be practical.

The Cloud outbound HTTP implementation is significantly different from Spin's, so this feature may require a bit more than just porting.

## `containerd` shim

The `containerd` shim uses the Spin outbound HTTP implementation, so this should Just Work, but we may need to do some analysis to ensure that it meets the scheduling requirements (all components on same node).

The shim links all triggers to be in-process: I am not sure if this has any implications for the "host the HTTP engine inside the host component" strategy.

# Future possibilities

* In the "web of microservices," many chaining targets should be called only by other components, not by external entities.  Such "private endpoints" should be planned for even if they don't happen in the first cut of the design.
* Service chaining opens the door to structured middleware.  A component acting as middleware would, instead of making a request to a specific endpoint, make a request to "the next component in the chain."
* The initial proposal allows apps to express "this request be handled locally."  This suggests a possible future enhancement of being able to express that a request must be handled "nearby," e.g. same rack, same datacentre, same region.  This loosens scheduling while retaining at least some guarantees about performance and cost.

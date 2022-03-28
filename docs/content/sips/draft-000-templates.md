title = "SIP DRAFT-000 - Templates"
template = "main"
date = "2022-03-22T14:53:30Z"

---

Summary: Proposes changes to the `spin new` syntax and to the template
authoring format

Created: March 28, 2022

Status: DRAFT

## Background

We have an initial cut of a template system planned for 0.1, but design and UX
work on this was somewhat constrained by the 0.1 deadline.  The 0.1 design is
closely based on `cargo generate` because it was a readily available, er,
template in the language we were using.  This proposal discusses goals and
design options for taking the template system forward.

The existing work gives us a solid basis with a well-documented template syntax
using the Liquid engine, safe failure modes via staging, and other good
stuff.  No changes are proposed to that core.  This SIP relates to user
experience (for both consumers and authors) and broader architecture.

## Goals

* Convenient, forgiving user experience for consumers
* Foundation for delivery via IDE
* Minimise what authors need to know

## Should we have a template system at all?

It's worth considering this because there are a ton of existing templating
systems, some language specific (e.g. `cargo generate`, `dotnet new`), some
generic (e.g. Yeoman).  These are likely to be idiomatic to the languages
people use.  Is it worth having our own?

Reasons why we should:

* Some languages _don't_ have template support, or require the user to go
  install yet more tools.
* It allows documentation to assume Spin templates instead of having to
  write different docs for every language.
* Having our own system doesn't preclude offering Spin templates via
  language-idiomatic systems too.

Reasons why we shouldn't:

* It's a decent chunk of work to do it well.
* It's extra code for every copy of Spin to carry around.

I believe the general consensus is that the case for having templates built
in is compelling, but it's always worth checking that assumption.

## Identifying a template

Templates are currently identified by a repository ID and directory name.
Because Spin is a multi-language runtime, it would be good to also identify
templates by language, so that a user could easily see and choose templates
in their preferred language.

The question of the repository ID is what we want "adding" a template to mean.
Does adding a template:

* Record a mapping of a repository ID to a Git location
* Copy a template to the local system

At the moment it seems to do both, which I found a bit confusing. If we take
the view that a template is copied, then we no longer need the repository
ID and can address templates by their names along.

This, by the way, is the approach taken by the `dotnet` templating system,
where you run `dotnet new --install <package-name>` to install a template
package, and then use template by running `dotnet new <template-name>`,
with an optional `-lang` option to specify the language.

Because this does not qualify every template with a unique Git URL, it does
require some consideration of name clashes.  There may also be value in
allowing users to run templates without installing them.

## User experience: starting a template

The `spin new` command create a new Spin application.  (We may want to consider
also allowing it to create a new component in an existing application.)

Proposed syntax:

| Command        | Behaviour |
|----------------|-----------|
| `spin new`     | Prompt from installed templates, then run chosen template |
| `spin new http` | Runs the `http` template. If this is available in more than one language, prompts for language or uses default language |
| `spin new http -lang haskell` | Runs the Haskell `http` template |
| `spin new -lang haskell` | Prompt from installed Haskell-language templates, then run chosen template (maybe not useful?) |

### What if there are no templates?

We don't have an installer so we can't guarantee to have templates present the
first time the user runs `spin new`.  The `cargo generate` experience for this
is to print an error about a config file path; this is not good.

If the user runs `spin new` (with any options) and we do not have any templates
installed, we should offer to install the default ones from Spin itself.  We should
also point them at the `spin templates install` command to add more.

### Template search strategy

If the user runs `spin http foo`:

* If there is NO default language set:
  * If there ARE templates named `foo` installed:
    * Prompt for which language to use (of the ones available).
    * Always prompt even if there is only one option, so the user knows if it's going to meet their needs.
  * If there are NO templates named `foo` installed:
    * If there are no templates at all installed, offer to install the default templates as above
    * If there are templates installed:
      * Message that `foo` is not installed
      * List the templates that are installed, and point to `spin templates install`
* If there IS a default language set, OR the user explicitly specifies a language:
  * If there IS a template named `foo` in that language installed:
    * Run it
  * If there are templates named `foo` installed but not in the given language:
    * Message that `foo` is not installed for your default language `Haskell`
    * Prompt with the languages for which `foo` is installed
  * If there are NO templates named `foo` installed:
    * Same as if no default language

(Note: if the user runs the command in silent/no-interaction mode, then instead
of listing or prompting, we should just fail with an explanatory message.)

### Applications and components

Our current templates are of the "hello world" variety - they produce a small
but complete application, including the application metadata and a starter
component (code and wiring).

It would be good to provide a way to add a component to an existing application.
This is a little tricky because (a) it requires modification of an existing
file, not just copying a file; and (b) the component templates have to be
constrained by the trigger type of the application.

@michelleN has also proposed that it should be possible to create _only_ the
application metadata, up to and including the trigger type, but with no
components.  (The `spin up` command would then be modified to detect if
an app had not components and guide the user on how to add them.)  The
user experience for this is a bit tricky.  One option is to have separate
`spin init/spin new` verbs, but for some readers that will create ambiguity -
I want to create an application, which one is right for me?  Another is
to have flags on `spin new` to indicate "bare starter," "full starter,"
or "incremental add," but this creates ceremony and cognitive load. A
possible approach is to make it interactive - if no manifest exists,
ask whether to create a bare one or one with a starter component.  But
this may be a confusing question for users' first encounter with Spin!
"I don't know, I just want to work!"

_TODO: think about how bare and incremental `new` might impact the
authoring of templates as well as the consumer experience._

### Running a template without installing

This could be something like:

```
spin new https://github.com/fermyon/spin/templates/http/rust
```

There would be no language option.

We'd need to enforce that IDs couldn't look like URLs.

### What's this 'default language' thing?

A somewhat unformed idea that users who primarily use one language could set an
environment variable or config file setting and have the template system default
to using that.

## User experience: installing templates

The current install experience has two paths, remote and local.  The remote path
takes a Git repository URL and branch name, and clones the entire branch into
the local cache.  The local one copies a file tree into cache.  In both cases
you need to supply a repository name.

It would be good, if we can, to avoid doing a full git clone.  A repository
can be quite large, and we are really interested only in a snapshot of the
template files on the chosen branch.  We can avoid cloning history via the
`--depth 1` option but there doesn't seem to be a great option for only
copying the template files.

```
spin templates install --dir <PATH>
spin templates install --git <URL> [--branch foo]
```

The current convention for Git sources is that the templates should be located
at `/templates`.  I'm not sure whether directory sources expect a `templates`
subdirectory or if you specify the full path - I thought the former but it
seemed to work when I used the latter. But maybe I was wrong.

The output of `spin templates install` should be a list of installed templates.
By default this should be in human readable table format, e.g.:

```
Installed 3 template(s)

Name     Language     Description
sqs      Rust         Basic SQS queue handler
sqs      Go
sqs-db   Rust         SQS queue handler with scaffold for piping output into DynamoDB
```

but there should be a `-o json` option for programmatic consumption.

### Name clashes on installation

_TO DO_

### Updating installed templates

_TO DO_

## User experience: managing templates

_TO DO: words_

| Command | Behaviour |
|---------|-----------|
| `spin templates list` | Lists all templates |
| `spin templates list http` | Lists all templates named `http` (or containing the string `http`?) |
| `spin templates list -lang haskell` | Lists all Haskell-language templates |

Default output is tabular and similar to shown above, but again we should
offer `-o json` to ease parsing in IDEs or other tools.

If there are no templates then it could tell you how to install some from the
Spin Git repository.

## Template authoring: file format

The current system assumes a `spin-generate.toml` file at the root of the
template and intermingled with the content files; any scripts are part of
the same tree.  As the current templating system relies on file copying,
this requires special handling to exclude those non-content files.

This document proposes separating template metadata and instructions from
the template files/content.  E.g.:

```
+
|-+ metadata (or manifest or something)
| |- manifest.toml (or spin-generate or whatever)
| |- script_file.script
| |- anything_else
|-+ content (or assets or something)
  |- spin.toml
  |- src/...
```

This separation reduces the risk that metadata files may end up in the output,
and gives authors an unobstructed view of the content.

### Applications and components

We discussed above the possibility of having "bare app," "smol app," and
"incremental add" - create an app with no components, create an app with
one component, and add a component to an existing app.

If we do this, it requires further tweaking to the template format,
or possibly two categories of template:

* We need to be able to generate a `spin.toml` _only_.  Our triggers
  are currently simple enough that this hardly merits a template - it needs
  only a trigger type and parameter placeholders, the set of which
  is fixed based on the trigger.
* We need to be able to generate a component (source code and `spin.toml`
  entry) _only_.
* We need to be able to restrict template selection in the incremental
  scenario to only those templates compatible with the application trigger.
  Adding a Redis component to a HTTP application is, at least for now,
  an error.

We could of course make these two separate categories of template. But
then we need an easy way to do the "smol app" without the user having
to pick and choose.

A possible strategy here is:

* Templates are for components (there are no templates for metadata).
* Each template says what trigger it works with.
* If you `spin new foo`:
  * If there's a `spin.toml`, it checks that `foo` works with the
    current trigger and if so adds a new component based on the template.
  * If there's no `spin.toml`, it looks at the trigger that `foo` works
    with and creates a `spin.toml` with that as the application trigger.
    It gets the trigger parameters from _TO DO: waves hand_.  Then it
    adds the component as above.
* If you `spin new`:
  * If there's a `spin.toml`, it prompts you with a list of installed
    templates that work with the trigger in that `spin.toml`.
  * If there's no `spin.toml`, it prompts you with a list of installed
    templates BUT ALSO offers you "Empty application".  If you choose that,
    then  it prompts for the trigger, and creates the `spin.toml` using _hand
    waving intensifies_.
  * We could reserve the name `empty` so you could go `spin new empty`?

| Command             | `spin.toml` exists            | `spin.toml` doesn't exist         |
|---------------------|-------------------------------|-----------------------------------|
| `spin new`          | Prompt for compatible template      | Prompt for template OR empty; if template, use its trigger type; if empty, prompt for trigger type |
| `spin new foo`      | Add a `foo` component if compatible | Create with starter `foo` component     |
| ? `spin new empty` ? | Friendly error               | Prompt for trigger type           |

### Implementation detail: fiddly files

From the `yo wasm` experience we learned that some template content files
could be interpreted in unwanted ways by tooling. The specific instance we
ran into was the `.gitignore` file.  We could implement a standard
convention for working this, e.g. if a content file has the extension
`.spinstripsuffix` then that extension gets removed during copy.

### Parameter specification

Templates may have parameters. for which the user must supply values in order
to run the template. For 0.1 we implemented the parameter definitions using
a custom TOML format, but this isn't great for other tools such as IDEs to
parse and validate (we can do it, but it requires custom logic).  An alternative
approach could be to express the parameter definitions as a JSON Schema,
making for easier parsing into a user interface, and for convenient validation
via libraries such as `ajv`.

That said, from experience with `yo wasm`, it's not always feasible to
express a template's needs as a static set of parameters known at compile
time.  So longer term we may need a better way to define this interaction
anyway, and it may not be worth investing in JSON Schema.

Additionally, plain JSON Schema might not be enough - for example you may
want to map user-facing display text to something else on the back end.

## Template authoring: custom logic

The 0.1 template system originally supported pre- and post-processing via
`.rhai` files.  Rhai is a somewhat Rust-ish scripting language.  It's pretty
readable but by no means is it well-known.  If we want to offer custom
logic then it might be better to use some portable byte code format
that authors can create from the language of their choice.  Ideally we
would choose a secure byte code format so as to minimise risk to users
running semi-trusted templates.  Think, think, think.

The initial use case for custom logic in the pre-0.1 templates was to provide
an enum-like way to select between licence files. That should probably be
built in anyway.  So we should consider whether we need custom logic at all,
and if so what sort of use cases, before investing effort in this.  E.g.

* Transform user input before putting values in templates
* Customising which files are copied
* Displaying usage or post-setup messages

Notes:

* Yeoman is _all_ custom logic, with the runtime providing functions for
  common tasks like templated copy.  This is another approach but might be
  labour intensive, particularly with providing idiomatic bindings in
  different source languages.  (Yeoman was JS only.)
* `yo wasm` used custom logic to install tools, such as the Rust Wasm
  target.  That's much less relevant here I think?

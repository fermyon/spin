title = "Creating Spin templates"
template = "main"
date = "2022-03-14T00:22:56Z"
[extra]
url = "https://github.com/fermyon/spin/blob/main/docs/content/template-authoring.md"
---

Spin templates allow a Spin developer to quickly create the skeleton of an
application or component, ready for the application logic to be filled in.

A template consists of two directories, `content` and `metadata`.

* The `content` directory contains all the files you'd like to be copied into
  the Spin application directory, such as source code, the `spin.toml` file,
  standard assets, precompiled modules, etc.  These files can contain placeholders
  so the user of the template can customise the end result.
* The `metadata` directory contains the files the control how the template is
  instantiated.  In this version of Spin, the only file in this directory
  should be the _template manifest_.

For examples of the directory contents, see the `templates` directory in the
[Spin GitHub repository](https://github.com/fermyon/spin).

Templates must always be shared in a `templates` directory.  This allows the
installer to locate them in repos that contain other content.

## Authoring the content

Copy all the files that you want to be copied as part of the template into
the `content` directory. If you do nothing more, they will be copied
verbatim. Often, though, you'll want to allow the user to put their own
values in - for example, a project name, or a HTTP route.

To do this, replace the text you want the user to be able to substitute
with an expression of the form `{{parameter-name}}`, where `parameter-name`
is an identifier of your choice.  **You will need to add an entry to
the manifest matching this name** - see below.

You can reuse a parameter in more than one place - it will be prompted for
only once and will get the same value in each place.

You can also transform the user value by specifying a filter after a bar:
`{{parameter-name | filter-name}}`.  This is particularly useful when you
want to conform to specific language conventions. The following filters
are supported:

| Name          | Effect |
|---------------|--------|
| `kebab_case`  | Transforms input into kebab case, e.g. `My Application` to `my-application` |
| `snake_case`  | Transforms input into snake case, e.g. `My Application` to `my_application` |
| `pascal_case` | Transforms input into Pascal case, e.g. `my appplication` to `MyApplication` |

## Authoring the manifest

The template manifest is a TOML file. It must be named `spin-template.toml`.

```toml
manifest_version = "1"
id = "my-application"
description = "An application"

[parameters]
# Example parameter
project-name = { type = "string", prompt = "Project name" }
```

* `manifest_version` specifies the format this manifest follows. It must be `"1"`.
* `id` is however you want users to refer to your template in `spin new`.
  It may contain letters, digits, hypens and underscores.
* `description` is optional. It is shown when displaying the template.

The `parameters` table is where you list the placeholders that you edited
into your content for the user to substitute. You should include an entry
for each parameter. The key is the parameter name, and the value a JSON
document that contains at minimum a `type` and `prompt`.  `type` must
currently be `string`.  `prompt` is displayed when prompting the user
for the value to substitute.

The document may also have a `default`, which will be displayed to the user
and can be accepted by pressing Enter. It may also specify constraints
on what the user is allowed to enter. The following constraints are
supported:

| Key           | Value and usage |
|---------------|-----------------|
| `pattern`     | A regular expression. The user input must match the regular expression to be accepted. |

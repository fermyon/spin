# Project Spin

Project Spin is the next version of the Fermyon runtime.

## The Spin CLI

### Using templates and template repositories

```
➜ spin templates
spin-templates 0.1.0
Commands for working with WebAssembly component templates

USAGE:
    spin templates <SUBCOMMAND>

FLAGS:
    -h, --help    Prints help information

SUBCOMMANDS:
    add         Add a template repository locally
    generate    Generate a new project from a template
    help        Prints this message or the help of the given subcommand(s)
    list        List the template repositories configured

➜ spin templates add --name suborbital --git https://github.com/suborbital/subo
[2021-11-02T18:53:32Z DEBUG fermyon_templates] adding repository https://github.com/suborbital/subo to "/Users/radu/Library/Caches/spin/templates/suborbital"

➜ spin templates list
+------------------------------------------------------------------------------------+
| Name             Repository   URL                                  Branch          |
+====================================================================================+
| scc-k8s          suborbital   https://github.com/suborbital/subo   refs/heads/main |
| assemblyscript   suborbital   https://github.com/suborbital/subo   refs/heads/main |
| rust             suborbital   https://github.com/suborbital/subo   refs/heads/main |
| project          suborbital   https://github.com/suborbital/subo   refs/heads/main |
| swift            suborbital   https://github.com/suborbital/subo   refs/heads/main |
| scc-docker       suborbital   https://github.com/suborbital/subo   refs/heads/main |
+------------------------------------------------------------------------------------+

➜ spin templates add --name localtest --local crates/engine/tests/rust-echo
[2021-11-02T19:02:46Z DEBUG fermyon_templates] adding local template from "/Users/radu/projects/src/github.com/fermyon/spin/crates/engine/tests/rust-echo" to "/Users/radu/Library/Caches/spin/templates/local/templates/localtest"

➜ spin templates list
+------------------------------------------------------------------------------------+
| Name             Repository   URL                                  Branch          |
+====================================================================================+
| localtest        local                                                             |
| scc-k8s          suborbital   https://github.com/suborbital/subo   refs/heads/main |
| assemblyscript   suborbital   https://github.com/suborbital/subo   refs/heads/main |
| rust             suborbital   https://github.com/suborbital/subo   refs/heads/main |
| project          suborbital   https://github.com/suborbital/subo   refs/heads/main |
| swift            suborbital   https://github.com/suborbital/subo   refs/heads/main |
| scc-docker       suborbital   https://github.com/suborbital/subo   refs/heads/main |
+------------------------------------------------------------------------------------+
➜ spin templates generate --repo local --template localtest --path tst
```

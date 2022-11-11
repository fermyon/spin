# {{ site-title }}

_Built with (Bartholomew)[https://bartholomew.fermyon.dev/], (Fermyon)[https://www.fermyon.com]'s open source micro-CMS._

## Directory Structure:

- `config/site.toml`: The main configuration file for the site. You should edit this.
- `content/`: Your markdown files go in here.
- `scripts/` (advanced): If you want to write your owh Rhai scripts, they go here.
- `spin.toml`: The configuration file for the Spin application.
- `static/`: Static assets like images, CSS, and downloads go in here.
- `templates/`: Your handlebars templates go here. 

## Running the site

To start this site, run `spin up`` from this directory.  (If you don't have Spin, you can install it from (here)[https://developer.fermyon.com/spin/install].)

```console
$ spin up --follow-all
spin up --follow-all
Serving HTTP on address http://127.0.0.1:3000
Available Routes:
  bartholomew: http://127.0.0.1:3000 (wildcard)
  fileserver: http://127.0.0.1:3000/static (wildcard)
```

Now you can point your web browser to `http://localhost:3000/` and see your Bartholomew site.

## Publishing this site

You can publish this site to (Fermyon Cloud)[https://developer.fermyon.com/cloud/index] by running `spin deploy`.

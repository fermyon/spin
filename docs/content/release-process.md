title = "Creating a new Spin release"
template = "spin_main"
date = "2022-03-14T00:22:56Z"
[extra]
url = "https://github.com/fermyon/spin/blob/main/docs/content/release-process.md"

---

To cut a release of Spin, you will need to do the following:

1. Create a pull request that changes the version number for your new version
   (e.g. `1.1.0-pre0` could become either `1.0.1` for a patch release or
   `1.1.0` for a minor release)
   - Bump the version in Spin's `Cargo.toml`
   - Update SDK_VERSION in `templates/Makefile`
   - Run `make build` so that `Cargo.lock` and other associated files are updated

   The pull request should have a base of `main`, unless this is an additional
   pre-release for a major/minor version, e.g. `v1.0.0-rc.2`, in which case the
   base should be the release branch, e.g. `v1.0`.

1. Merge the PR created in #1 (Such PRs are still required to get approvals, so
   make sure you get signoff on the PR)

1. Before proceeding, verify that the merge commit intended to be
   tagged is green, i.e. CI is successful

1. If this is the first release for this major/minor version, create a release
   branch, e.g. `v1.1`. With our branch protection rules this is easiest from
   the Github UI with the
   [New Branch button here](https://github.com/fermyon/spin/branches).

1. Switch to the release branch locally and create a new tag with a `v` and
   then the version number, e.g. `v1.1.0`. Then, push the tag to the
   `fermyon/spin` origin repo.

   As an example, via the `git` CLI:

   ```
   # Switch to the release branch
   git checkout v1.1
   git pull

   # Create a GPG-signed and annotated tag
   git tag -s -m "Spin v1.1.0" v1.1.0

   # Push the tag to the remote corresponding to fermyon/spin (here 'origin')
   git push origin v1.1.0
   ```

1. Unless this is a pre-release, switch back to `main` and update the
   `Cargo.toml` and `templates/Makefile` versions again, this time to
   e.g. `1.2.0-pre0` if `1.2.0` is the next anticipated release.
   - Run `make build` so that `Cargo.lock` and other associated files are updated
   - PR this to `main`
   - See [sips/011-component-versioning.md](sips/011-component-versioning.md)
     for details

1. The Go SDK tag associated with this release (e.g. `sdk/go/v1.1.0`) will be
   created in the [release action] that has been triggered by the tag push.

1. When the [release action] completes, binary artifacts and checksums will be
   automatically uploaded to the GitHub release.

1. A Pull Request will also be created by `fermybot` containing changes to the
   templates per the updated SDK version. If this is a pre-release for a
   major/minor version, be sure to change the base of the PR from `main` to the
   release branch, e.g. `v1.1`. Once CI completes, approve this PR and merge
   via a merge commit (rather than squash or rebase).
   
   This will trigger the `push-templates-tag` job in the [release action],
   pushing the `spin/templates/v0.9` tag. (Note that this tag may be
   force-pushed for all patch releases of a given minor release.)

1. Go to the GitHub [tags page](https://github.com/fermyon/spin/releases),
   edit the release and add the release notes.

1. Be sure to include instructions for
   [verifying the signed Spin binary](./sips/012-signing-spin-releases.md). The
   `--certificate-identity` value should match this release, e.g.
   `https://github.com/fermyon/spin/.github/workflows/release.yml@refs/tags/v1.1.0`.

The release is now complete!

[release action]: https://github.com/fermyon/spin/actions/workflows/release.yml

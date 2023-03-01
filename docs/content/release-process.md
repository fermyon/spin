title = "Creating a new Spin release"
template = "spin_main"
date = "2022-03-14T00:22:56Z"
[extra]
url = "https://github.com/fermyon/spin/blob/main/docs/content/release-process.md"

---

To cut a release of Spin, you will need to do the following:

1. Create a pull request that changes the version number for your new version
   (e.g. `0.10.0-pre0` could become either `0.9.1` for a patch release or
   `0.10.0` for a "major" release)
   - Bump the version in Spin's `Cargo.toml`
   - Update SDK_VERSION in `templates/Makefile`
   - Check the docs for hard-coded version strings
1. Merge the PR created in #1 (Such PRs are still required to get approvals, so
   make sure you get signoff on the PR)
1. Before proceeding, verify that the merge commit on `main` intended to be
   tagged is green, i.e. CI is successful
1. Create a new tag with a `v` and then the version number (`v0.9.1`)
1. Update the `Cargo.toml` and `templates/Makefile` versions again, this
   time to e.g. `0.11.0-pre0` if `0.11.0` is the next anticipated release.
   - PR this to main
   - See [sips/011-component-versioning.md](sips/011-component-versioning.md)
     for details
1. The Go SDK tag `sdk/go/v0.9.1` will be created in the [release action].
1. When the [release action] completes, binary artifacts and checksums will be
   automatically uploaded to the GitHub release.
1. A Pull Request will also be created by `fermybot` containing changes to the
   templates per the updated SDK version. Once CI completes, approve this PR and
   merge via a merge commit. This will trigger the `push-templates-tag` job in
   the [release action], pushing the `spin/templates/v0.9` tag. (Note
   that this tag may be force-pushed for all patch releases of a given minor release.)
1. Go to the GitHub [tags page](https://github.com/fermyon/spin/releases),
   edit a release, add the release notes.

At this point, you can verify in the GitHub UI that the release was successful.

[release action]: https://github.com/fermyon/spin/actions/workflows/release.yml

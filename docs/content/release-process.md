title = "Creating a new Spin release"
template = "main"
date = "2022-03-14T00:22:56Z"
[extra]
url = "https://github.com/fermyon/spin/blob/main/docs/content/release-process.md"
---

To cut a release of Spin, you will need to do the following:

1. Create a pull request that changes the version number for your new version
   (e.g. 0.1.0 becomes 0.1.1)
   - `Cargo.toml` is the most important place to make this change
   - Check the docs for hard-coded version strings
1. Merge the PR created in #1 (Such PRs are still required to get approvals, so
   make sure you get signoff on the PR)
1. Before proceeding, verify that the merge commit on `main` intended to be
   tagged is green, i.e. CI is successful
1. Create a new tag with a `v` and then the version number (`v0.1.1`)
1. Push the tag up to `main` on GitHub
   - This will trigger a release build
1. Wait for the `release`
   [action](https://github.com/fermyon/spin/actions/workflows/release.yaml) to
   complete, binary artifacts and checksums will be automatically uploaded.
1. Go to the GitHub [tags page](https://github.com/fermyon/spin/releases),
   edit a release, add the release notes.

At this point, you can verify in the GitHub UI that the release was successful.

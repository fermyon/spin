# Triage Duty Guide

This guide explains the goals of triage duty and outlines the triage process.

## Goals

1. Identify and properly manage critical issues in a timely manner.
2. Label and prioritize incoming issues.
3. Help move along conversations in issues until they are scoped for the backlog and ready for someone to pick up, closed or resolved.

## Steps

1. Add all new issues to the [`Spin Triage` project board](https://github.com/orgs/fermyon/projects/7/) under the `Triage Needed` column.

To do this:

- Go to the [project board](https://github.com/orgs/fermyon/projects/7/) and select `+ Add Item` at the bottom of the `Triage Needed` column.
- Select `+`
- Select `Add items to project`
- In the search bar, filter issues by `is:issue is:open`
- Select all and push the button `add selected items`

2. For each issue in the `Triage Needed` column, add appropriate labels and move the issue to another column if the issue is ready for a different column (`Investigating / Open for Comment`, `Backlog`, `In progress`) or close.

To do this:

- Determine if the issue is a `bug`, `enhancement`, or `question` and label as such.
  - If the issue does not clearly fall into one of these buckets, please ask more questions to determine what labels make sense.
  - Please bubble up and help resolve any critical bugs as you come across them.
  - If a `question` exposes a need for an improvement in the docs, please open up an issue in the [developer docs repo](https://github.com/fermyon/developer/issues).
- If the issue is being currently investigated, move the issue to the `Investigating / Open for Comment` column and assign the issue an owner (the person who is investigating).
- If the issue is an enhancement and we do not know yet whether we want to address it or it needs more input and discussion, move the issue to the `Investigating / Open for Comment` column. Please also bring this up in the next Spin maintainer or community meeting.
- If the issue is well scoped, we want to resolve it, and it is ready to be picked up, move it to the `Backlog` column.
- If the issue is being actively worked on, please ensure there is an owner and move it to the `In Progress` column.
- If the issue requires no further action or it is a suggestion we don't plan on working on, add an explanation, link to other relevant issues/comments, add the appropriate labels (`duplicate`, `wontfix`) and then close the issue.

3. Visit the [Security Vulnerability tab](https://github.com/fermyon/spin/security/dependabot) to see if there are any outstanding dependabot PRs to review or if any vulnerabilities need to be addressed.

If merging a dependabot PR turns out to be a complicated endevaor and there are reasons for not being able to merge it immediately, leave a comment explaining the situation and where application, link to relevant upstream issues/PRs to watch for progress.

4. Time permitting, review and help move along issues in the `Investigating / Open for Comment` column.

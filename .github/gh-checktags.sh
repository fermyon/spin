#!/usr/bin/env bash

## for the release tags v*, check if
## - spin templates tag exists

set -e

which gh &> /dev/null || {
  echo "This script requires the Github CLI 'gh'"
  exit 1
}

tags=$(gh release list --repo fermyon/spin --exclude-pre-releases --exclude-drafts | grep -v TITLE | awk '{print $1}')

exit_code=0
for tag in $tags; do
  if [[ $tag != v* ]]; then
    continue
  fi

  if [[ $tag == *-rc* ]]; then
    continue
  fi

  # remove trailing .\d+ from the version
  major_minor=`echo $tag | sed 's/\.[0-9]*$//g'`

  # check template tag
  if [[ -z "$(git tag -l spin/templates/$major_minor)" ]]; then
    echo "tag spin/templates/$major_minor does not exist"
    exit_code=1
  fi
done

exit $exit_code

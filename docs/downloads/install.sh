#!/usr/bin/env bash
set -euo pipefail

# Fancy colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color aka reset

# Version to install. Defaults to latest or set by --version or -v
VERSION=""

# Print in colors - 0=green, 1=red, 2=neutral
# e.g. fancy_print 0 "All is great"
fancy_print() {
    if [[ $1 == 0 ]]; then
        echo -e "${GREEN}${2}${NC}"
    elif [[ $1 == 1 ]]; then
        echo -e "${RED}${2}${NC}"
    else
        echo -e "${2}"
    fi
}

# Function to print the help message
print_help() {
    fancy_print 2 ""
    fancy_print 2 "---- Spin Installer Script ----"
    fancy_print 2 "This script installs Spin in the current directory."
    fancy_print 2 ""
    fancy_print 2 "Comand line arguments"
    fancy_print 2 "--version or -v  : Provide what version to install e.g. \"v0.5.0\" or \"canary\"."
    fancy_print 2 "--help    or -h  : Shows this help message"
}

# Function used to check if utilities are available
require() {
    if ! hash "$1" &>/dev/null; then
        fancy_print 1 "'$1' not found in PATH. This is required for this script to work."
        exit 1
    fi
}

# Parse input arguments
while [[ $# -gt 0 ]]; do
    case $1 in
    '--version' | -v)
        shift
        if [[ $# -ne 0 ]]; then
            VERSION="${1}"
        else
            fancy_print 1 "Please provide the desired version. e.g. --version v0.5.0 or -v canary"
            exit 0
        fi
        ;;
    '--help' | -h)
        shift
        print_help
        ;;
    *)
        fancy_print 1 "Unknown argument ${1}."
        print_help
        exit 1
        ;;
    esac
    shift
done

# Check all required utilities are available
require curl
require tar
require uname

# Check if we're on a suppoerted system and get OS and processor architecture to download the right version
UNAME_ARC=$(uname -m)

case $UNAME_ARC in
"x86_64")
    ARC="amd64"
    ;;
"arm64" | "aarch64")
    ARC="aarch64"
    ;;
*)
    fancy_print 1 "The Processor type: ${UNAME_ARC} is not yet supported by Spin."
    exit 1
    ;;
esac

case $OSTYPE in
"linux-gnu"*)
    OS="linux"
    if [[ $ARC == "aarch64" ]]; then
        fancy_print 1 "The Processor type: ${ARC}, on ${OSTYPE} is not yet supported by Spin."
        exit 1
    fi
    ;;
"darwin"*)
    OS="macos"
    ;;
*)
    fancy_print 1 "The OSTYPE: ${OSTYPE} is not supported by this script."
    fancy_print 2 "Please refer to this article to install Spin: https://spin.fermyon.dev/quickstart/"
    exit 1
    ;;
esac

# Check desired version. Default to latest if no desired version was requested
if [[ $VERSION = "" ]]; then
    VERSION=$(curl -so- https://github.com/fermyon/spin/releases | grep 'href="/fermyon/spin/releases/tag/v[0-9]*.[0-9]*.[0-9]*\"' | sed -E 's/.*\/fermyon\/spin\/releases\/tag\/(v[0-9\.]+)".*/\1/g' | head -1)
fi

# Constructing download FILE and URL
FILE="spin-${VERSION}-${OS}-${ARC}.tar.gz"
URL="https://github.com/fermyon/spin/releases/download/${VERSION}/${FILE}"

# Download file, exit if not found - e.g. version does not exist
fancy_print 0 "Step 1: Downloading: ${URL}"
curl -sOL --fail $URL || (fancy_print 1 "The requested file does not exist: ${FILE}"; exit 1)
fancy_print 0 "Done...\n"

# Decompress the file
fancy_print 0 "Step 2: Decompressing: ${FILE}"
tar xfv $FILE
./spin --version
fancy_print 0 "Done...\n"

# Remove the compressed file
fancy_print 0 "Step 3: Removing the downloaded tarball"
rm $FILE
fancy_print 0 "Done...\n"

# Direct to quicks-start doc
fancy_print 0 "You're good to go. Check here for the next steps: https://spin.fermyon.dev/quickstart/"
fancy_print 0 "Run './spin' to get started"

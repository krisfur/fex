#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Detect container runtime
if command -v podman &> /dev/null; then
    RUNTIME="podman"
elif command -v docker &> /dev/null; then
    RUNTIME="docker"
else
    echo "Error: Neither docker nor podman found"
    echo "Install one of them to test other distros"
    exit 1
fi

usage() {
    echo "Usage: $0 <distro>"
    echo ""
    echo "Available distros:"
    for f in "$SCRIPT_DIR"/Dockerfile.*; do
        name="${f##*.}"
        echo "  $name"
    done
    echo ""
    echo "Example: $0 ubuntu"
    exit 1
}

if [ -z "$1" ]; then
    usage
fi

DISTRO="$1"
DOCKERFILE="$SCRIPT_DIR/Dockerfile.${DISTRO}"

if [ ! -f "$DOCKERFILE" ]; then
    echo "Error: Dockerfile not found for '$DISTRO'"
    echo "Expected: $DOCKERFILE"
    usage
fi

echo "Using $RUNTIME..."
echo "Building fex for $DISTRO..."
$RUNTIME build -t "fex-$DISTRO" -f "$DOCKERFILE" "$PROJECT_DIR"

echo ""
echo "Running container (interactive)..."
echo "Type 'fex' to launch the TUI, Ctrl+X to quit"
echo ""

case "$DISTRO" in
    snap)
        # snapd requires systemd as PID 1; run detached then exec in.
        CONTAINER="fex-snap-$$"
        $RUNTIME run -d --privileged --name "$CONTAINER" "fex-$DISTRO"
        echo "Waiting for snapd to start..."
        sleep 5
        $RUNTIME exec -it "$CONTAINER" /bin/bash
        $RUNTIME rm -f "$CONTAINER" 2>/dev/null
        ;;
    *)
        $RUNTIME run -it --rm "fex-$DISTRO"
        ;;
esac

#!/bin/bash

# Metaplex Convenience Wrapper
# Simple wrapper for the Metaplex management script

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
METAPLEX_SCRIPT="$SCRIPT_DIR/metaplex/manage_metaplex.sh"

# Check if the manage_metaplex.sh script exists
if [ ! -f "$METAPLEX_SCRIPT" ]; then
    echo "❌ Metaplex management script not found at: $METAPLEX_SCRIPT"
    exit 1
fi

# Forward all arguments to the actual script
exec "$METAPLEX_SCRIPT" "$@" 
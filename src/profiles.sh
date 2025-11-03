#!/bin/bash

set -e

# Profiles directory in /usr/local/lib/green-languages
PROFILES_DIR="/usr/local/lib/green-languages/glp.d"

check_tlp() {
    if ! command -v tlp &> /dev/null; then
        echo "Error: tlp is not installed or not in PATH"
        return 1
    fi
}

show_usage() {
    cat << EOF
Usage: glp [COMMAND]

Commands:
  list              List all available profiles
  enable PROFILE    Enable a profile (restarts TLP)
  help              Show this help message

Profiles are stored in: ${PROFILES_DIR}/
Each profile should be a .conf file containing TLP settings.

EOF
}

list_profiles() {
    if [ ! -d "$PROFILES_DIR" ]; then
        echo "Error: glp.d directory not found at $PROFILES_DIR"
        return 1
    fi

    local profiles=($(find "$PROFILES_DIR" -maxdepth 1 -name "*.conf" -type f -printf '%f\n' | sed 's/\.conf$//'))

    if [ ${#profiles[@]} -eq 0 ]; then
        echo "No profiles found in $PROFILES_DIR"
        return 1
    fi

    echo "Available profiles:"
    for profile in "${profiles[@]}"; do
        echo "  - $profile"
    done
}

enable_profile() {
    local profile=$1

    if [ -z "$profile" ]; then
        echo "Error: Profile name required"
        return 1
    fi

    local profile_file="$PROFILES_DIR/$profile.conf"

    if [ ! -f "$profile_file" ]; then
        echo "Error: Profile '$profile' not found at $profile_file"
        return 1
    fi

    echo "Enabling profile: $profile"

    # Remove any previously enabled profile
    sudo rm -f /etc/tlp.d/*.conf

    # Copy profile to tlp.d with .conf extension
    sudo cp "$profile_file" "/etc/tlp.d/$profile.conf"
    sudo tlp start

    echo "Profile '$profile' enabled and TLP restarted"
}

main() {
    local command=$1

    case "$command" in
        list)
            check_tlp || return 1
            list_profiles
            ;;
        enable)
            check_tlp || return 1
            enable_profile "$2"
            ;;
        help|--help|-h)
            show_usage
            ;;
        *)
            echo "Unknown command: $command"
            show_usage
            exit 1
            ;;
    esac
}

main "$@"

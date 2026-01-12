#!/bin/bash

set -e

PROFILES_DIR="$GL_LIB_DIR/profiles/"

check_tlp() {
    if ! command -v tlp &> /dev/null; then
        echo "Error: tlp is not installed or not in PATH"
        return 1
    fi
}

show_usage() {
    cat << EOF
Usage: green-languages-profiles [COMMAND] [SUBCOMMAND] [OPTIONS]

Profile Management:
  profile list       List all available TLP profiles
  profile PROFILE    Enable a TLP profile (restarts TLP)

CPU Management:
  cpu disable [PCT]  Disable PCT% of CPUs (default: 100%)
  cpu enable [all]   Re-enable all CPUs (default behavior)
  cpu disable ht     Disable hyperthreading
  cpu enable ht      Re-enable hyperthreading
  cpu disable cs     Disable all C-states
  cpu enable cs      Re-enable all C-states

OS Management:
  aslr disable       Disable ASLR (Address Space Layout Randomization)
  aslr enable        Enable ASLR
  cache drop [LEVEL] Drop OS caches (default: 3 = all)

General:
  help               Show this help message

Examples:
  green-languages-profiles profile list
  green-languages-profiles profile powersave
  green-languages-profiles cpu disable 50
  green-languages-profiles cpu disable
  green-languages-profiles cpu enable
  green-languages-profiles cpu disable ht
  green-languages-profiles cpu enable cs
  green-languages-profiles aslr disable
  green-languages-profiles cache drop
  green-languages-profiles cache drop 1

Cache Levels:
  1 = Free pagecache only
  2 = Free dentries and inodes only
  3 = Free pagecache, dentries and inodes (default)

Profiles are stored in: ${PROFILES_DIR}/

EOF
}

list_profiles() {
    if [ ! -d "$PROFILES_DIR" ]; then
        echo "Error: green-languages-profiles.d directory not found at $PROFILES_DIR"
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

    sudo rm -f /etc/tlp.d/*.conf

    sudo cp "$profile_file" "/etc/tlp.d/$profile.conf"
    sudo tlp start

    echo "Profile '$profile' enabled and TLP restarted"
}

disable_cpus() {
    local percentage=$1

    if [ -z "$percentage" ]; then
        percentage=100
    fi

    if ! [[ "$percentage" =~ ^[0-9]+$ ]] || [ "$percentage" -lt 0 ] || [ "$percentage" -gt 100 ]; then
        echo "Error: Percentage must be between 0 and 100"
        return 1
    fi

    local available_cpus=()
    for cpu_path in /sys/devices/system/cpu/cpu[1-9]*/online; do
        if [ -f "$cpu_path" ]; then
            local online_status=$(cat "$cpu_path" 2>/dev/null)
            if [ "$online_status" = "1" ]; then
                local cpu_num=$(echo "$cpu_path" | grep -oP 'cpu\K[0-9]+')
                available_cpus+=("$cpu_num")
            fi
        fi
    done

    local total_available=${#available_cpus[@]}

    if [ $total_available -eq 0 ]; then
        echo "No CPUs available to disable (CPU 0 is always kept online)"
        return 1
    fi

    local num_to_disable=$(( (total_available * percentage + 99) / 100 ))

    if [ $num_to_disable -eq 0 ]; then
        echo "No CPUs to disable at ${percentage}%"
        return 0
    fi

    if [ $num_to_disable -gt $total_available ]; then
        num_to_disable=$total_available
    fi

    echo "Disabling $num_to_disable out of $total_available available CPUs (${percentage}%)..."

    local count=0
    for ((i=0; i<num_to_disable; i++)); do
        local cpu_num=${available_cpus[$i]}
        echo 0 | sudo tee "/sys/devices/system/cpu/cpu${cpu_num}/online" > /dev/null 2>&1 && ((count++)) || true
    done

    echo "Disabled $count CPUs (CPU 0 is restricted)"
}

enable_cpus() {
    echo "Re-enabling all CPUs..."

    for cpu_path in /sys/devices/system/cpu/cpu*/online; do
        echo 1 | sudo tee "$cpu_path" > /dev/null 2>&1 || true
    done

    echo "All CPUs re-enabled"
}

disable_cs() {
    echo "Disabling all C-states on all CPUs..."

    local count=0
    for state_disable in /sys/devices/system/cpu/cpu*/cpuidle/state*/disable; do
        if [ -f "$state_disable" ]; then
            echo 1 | sudo tee "$state_disable" > /dev/null 2>&1 && ((count++)) || true
        fi
    done

    echo "Disabled $count C-state entries across all CPUs"
}

enable_cs() {
    echo "Re-enabling all C-states on all CPUs..."

    local count=0
    for state_disable in /sys/devices/system/cpu/cpu*/cpuidle/state*/disable; do
        if [ -f "$state_disable" ]; then
            echo 0 | sudo tee "$state_disable" > /dev/null 2>&1 && ((count++)) || true
        fi
    done

    echo "Enabled $count C-state entries across all CPUs"
}

disable_ht() {
    if [ ! -f "/sys/devices/system/cpu/smt/control" ]; then
        echo "Error: SMT control interface not available on this kernel"
        return 1
    fi

    echo "Disabling hyperthreading (SMT)..."
    echo off | sudo tee /sys/devices/system/cpu/smt/control > /dev/null
    echo "Hyperthreading disabled"
}

enable_ht() {
    if [ ! -f "/sys/devices/system/cpu/smt/control" ]; then
        echo "Error: SMT control interface not available on this kernel"
        return 1
    fi

    echo "Enabling hyperthreading (SMT)..."
    echo on | sudo tee /sys/devices/system/cpu/smt/control > /dev/null
    echo "Hyperthreading enabled"
}

disable_aslr() {
    if [ ! -f "/proc/sys/kernel/randomize_va_space" ]; then
        echo "Error: ASLR control interface not available"
        return 1
    fi

    echo "Disabling ASLR..."
    echo 0 | sudo tee /proc/sys/kernel/randomize_va_space > /dev/null
    echo "ASLR disabled"
}

enable_aslr() {
    if [ ! -f "/proc/sys/kernel/randomize_va_space" ]; then
        echo "Error: ASLR control interface not available"
        return 1
    fi

    echo "Enabling ASLR..."
    echo 2 | sudo tee /proc/sys/kernel/randomize_va_space > /dev/null
    echo "ASLR enabled"
}

drop_caches() {
    local level=$1

    if [ -z "$level" ]; then
        level=3
    fi

    if ! [[ "$level" =~ ^[1-3]$ ]]; then
        echo "Error: Cache level must be 1, 2, or 3"
        echo "  1 = Free pagecache"
        echo "  2 = Free dentries and inodes"
        echo "  3 = Free pagecache, dentries and inodes (default)"
        return 1
    fi

    echo "Dropping caches (level $level)..."
    sync
    echo "$level" | sudo tee /proc/sys/vm/drop_caches > /dev/null
    echo "Caches dropped"
}

main() {
    local command=$1
    local subcommand=$2
    local arg3=$3

    case "$command" in
        profile)
            check_tlp || return 1
            if [ "$subcommand" = "list" ]; then
                list_profiles
            elif [ -z "$subcommand" ]; then
                echo "Error: Profile name required"
                echo "Use 'green-languages-profiles profile list' or 'green-languages-profiles profile PROFILE_NAME'"
                exit 1
            else
                enable_profile "$subcommand"
            fi
            ;;
        cpu)
            case "$subcommand" in
                disable)
                    if [ "$arg3" = "ht" ]; then
                        disable_ht
                    elif [ "$arg3" = "cs" ]; then
                        disable_cs
                    else
                        disable_cpus "$arg3"
                    fi
                    ;;
                enable)
                    if [ "$arg3" = "ht" ]; then
                        enable_ht
                    elif [ "$arg3" = "cs" ]; then
                        enable_cs
                    else
                        enable_cpus
                    fi
                    ;;
                *)
                    echo "Unknown cpu subcommand: $subcommand"
                    echo "Available: disable [PCT|ht|cs], enable [ht|cs]"
                    exit 1
                    ;;
            esac
            ;;
        aslr)
            case "$subcommand" in
                disable)
                    disable_aslr
                    ;;
                enable)
                    enable_aslr
                    ;;
                *)
                    echo "Unknown aslr subcommand: $subcommand"
                    echo "Available: disable, enable"
                    exit 1
                    ;;
            esac
            ;;
        cache)
            case "$subcommand" in
                drop)
                    drop_caches "$arg3"
                    ;;
                *)
                    echo "Unknown cache subcommand: $subcommand"
                    echo "Available: drop [LEVEL]"
                    exit 1
                    ;;
            esac
            ;;
        help|--help|-h|"")
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

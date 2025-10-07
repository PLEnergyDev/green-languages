from dataclasses import dataclass
from typing import Any
import subprocess
import argparse
import json
import sys
import os

from util import *


class Cpu:
    def __init__(self, value: int) -> None:
        self.cpu_path = f"/sys/devices/system/cpu/cpu{value}"
        if os.path.isdir(self.cpu_path):
            self.value = value
        else:
            raise ProgramError(f"Cpu {value} doesn't exist.")

    @property
    def enabled(self) -> bool:
        path = f"{self.cpu_path}/online"
        if os.path.exists(path):
            with open(path, "r") as file:
                value = file.read().strip()
                return value == "1"
        return True

    @enabled.setter
    def enabled(self, value: bool) -> None:
        path = f"{self.cpu_path}/online"
        if self.value != 0:
            write_file_sudo("1" if value else "0", path)

    @property
    def hyperthread(self) -> bool:
        path = f"{self.cpu_path}/topology/thread_siblings_list"

        try:
            siblings_str = read_file(path)
        except ProgramError:
            return False

        siblings = []
        for part in siblings_str.split(","):
            part = part.strip()
            if "-" in part:
                start, end = part.split("-")
                siblings.extend([str(i) for i in range(int(start), int(end) + 1)])
            elif part:
                siblings.append(part)

        siblings = sorted(siblings, key=int)
        if len(siblings) < 2:
            return False

        return str(self.value) in siblings[1:]

    @property
    def governor(self) -> str:
        path = f"{self.cpu_path}/cpufreq/scaling_governor"
        return read_file(path)

    @governor.setter
    def governor(self, value: str) -> None:
        path = f"{self.cpu_path}/cpufreq/scaling_governor"
        if value not in self.available_governors:
            raise ProgramError(f"governor '{value}' not available on CPU {self.value}.")
        write_file_sudo(value, path)

    @property
    def available_governors(self) -> list[str]:
        path = f"{self.cpu_path}/cpufreq/scaling_available_governors"
        return read_file(path).split()

    @property
    def min_hw_freq(self) -> int:
        path = f"{self.cpu_path}/cpufreq/cpuinfo_min_freq"
        return int(read_file(path))

    @property
    def max_hw_freq(self) -> int:
        path = f"{self.cpu_path}/cpufreq/cpuinfo_max_freq"
        return int(read_file(path))

    @property
    def min_freq(self) -> int:
        path = f"{self.cpu_path}/cpufreq/scaling_min_freq"
        return int(read_file(path))

    @min_freq.setter
    def min_freq(self, value: int) -> None:
        hw_min = self.min_hw_freq
        hw_max = self.max_hw_freq
        path = f"{self.cpu_path}/cpufreq/scaling_min_freq"
        if not (hw_min <= value <= hw_max):
            raise ProgramError(
                f"frequency {value} cannot be outside hardware limits [{hw_min}, {hw_max}]"
            )
        write_file_sudo(str(value), path)

    @property
    def max_freq(self) -> int:
        path = f"{self.cpu_path}/cpufreq/scaling_max_freq"
        return int(read_file(path))

    @max_freq.setter
    def max_freq(self, value: int) -> None:
        hw_min = self.min_hw_freq
        hw_max = self.max_hw_freq
        path = f"{self.cpu_path}/cpufreq/scaling_max_freq"
        if not (hw_min <= value <= hw_max):
            raise ProgramError(
                f"frequency {value} cannot be outside hardware limits [{hw_min}, {hw_max}]"
            )
        write_file_sudo(str(value), path)


def get_cpu_vendor() -> str:
    cpuinfo = read_file("/proc/cpuinfo")
    if "GenuineIntel" in cpuinfo:
        return "intel"
    if "AuthenticAMD" in cpuinfo:
        return "amd"
    raise ProgramError("Unknown CPU vendor")


def get_cpus(value: str) -> list[Cpu]:
    available_modes = ["online", "offline", "present", "possible"]
    if value not in available_modes:
        raise ProgramError(f"can only get {','.join(available_modes)} CPUs")

    cpus: list[Cpu] = []
    content = read_file(f"/sys/devices/system/cpu/{value}")
    if not content:
        return []

    for part in content.split(","):
        rng = part.split("-")
        if len(rng) == 2:
            cpus.extend([Cpu(v) for v in range(int(rng[0]), int(rng[1]) + 1)])
        else:
            cpus.append(Cpu(int(rng[0])))
    return cpus


def get_aslr() -> int:
    val = read_file("/proc/sys/kernel/randomize_va_space")
    return int(val)


def set_aslr(value: int) -> None:
    if value not in [0, 1, 2]:
        raise ProgramError(f"unsupported ASLR mode {value}")
    write_file_sudo(str(value), "/proc/sys/kernel/randomize_va_space")


def get_intel_boost() -> bool:
    path = "/sys/devices/system/cpu/intel_pstate/no_turbo"
    value = read_file(path)
    return not (value == "1")


def set_intel_boost(enable: bool) -> None:
    path = "/sys/devices/system/cpu/intel_pstate/no_turbo"
    if not os.path.exists(path):
        raise ProgramError(f"file {path} doesn't exist")
    write_file_sudo("0" if enable else "1", path)


def set_drop_caches(value: int = 3) -> None:
    """
    mode = 1 page cache only
    mode = 2 dentries & inodes only
    mode = 3 both (default)
    """
    if value not in [1, 2, 3]:
        raise ProgramError(f"unsupported drop_cache mode {value}")

    try:
        subprocess.run(["sync"], check=True)
    except subprocess.CalledProcessError as ex:
        raise ProgramError(f"failed while synchronizing - {ex}")
    write_file_sudo(str(value), "/proc/sys/vm/drop_caches")


def get_swaps() -> list[str]:
    devices = []
    try:
        if os.path.exists("/proc/swaps"):
            with open("/proc/swaps") as f:
                next(f, None)
                for line in f:
                    fields = line.split()
                    if fields:
                        devices.append(fields[0])
        return devices
    except Exception as ex:
        raise ProgramError(f"failed while getting swap - {ex}")


def set_swaps(enable: bool, devices: list[str] | None = None) -> None:
    try:
        if devices:
            for dev in devices:
                if enable:
                    subprocess.run(["sudo", "swapon", dev], check=True)
                else:
                    subprocess.run(["sudo", "swapoff", dev], check=True)
        else:
            if enable:
                subprocess.run(["sudo", "swapon", "-a"], check=True)
            else:
                subprocess.run(["sudo", "swapoff", "-a"], check=True)
    except Exception as ex:
        raise ProgramError(f"failed while setting swap - {ex}")


BACKUP_FILE = ".env_backup.json"


@dataclass
class Environment:
    """Controls Linux-specific OS environment"""

    def record_original(self):
        config = {}
        config["aslr"] = get_aslr()

        if get_cpu_vendor() == "intel":
            try:
                config["intel_boost"] = get_intel_boost()
            except ProgramError:
                config["intel_boost"] = None
        else:
            config["intel_boost"] = None

        config["cpus"] = {}
        for cpu in get_cpus("present"):
            if cpu.enabled:
                config["cpus"][cpu.value] = {
                    "enabled": True,
                    "governor": cpu.governor,
                    "max_freq": cpu.max_freq,
                    "min_freq": cpu.min_freq,
                }
            else:
                config["cpus"][cpu.value] = {"enabled": False}

        config["swaps"] = get_swaps()

        with open(BACKUP_FILE, "w") as f:
            json.dump(config, f, indent=2)

    def restore_original(self):
        if not os.path.exists(BACKUP_FILE):
            raise ProgramError(f"Backup file {BACKUP_FILE} not found")

        with open(BACKUP_FILE, "r") as f:
            config = json.load(f)

        set_aslr(config["aslr"])

        if get_cpu_vendor() == "intel" and config["intel_boost"] is not None:
            try:
                set_intel_boost(config["intel_boost"])
            except ProgramError:
                pass

        for cpu in get_cpus("present"):
            cpu_value_str = str(cpu.value)
            if cpu_value_str not in config["cpus"]:
                continue

            orig_cpu = config["cpus"][cpu_value_str]
            if orig_cpu["enabled"]:
                cpu.enabled = True
                cpu.governor = orig_cpu["governor"]
                cpu.max_freq = orig_cpu["max_freq"]
                cpu.min_freq = orig_cpu["min_freq"]
            else:
                cpu.enabled = False

        set_swaps(False)
        if config["swaps"]:
            set_swaps(True, config["swaps"])

    def __enter__(self):
        self.record_original()
        self.enter()
        return self

    def __exit__(
        self, exc_type: type | None, exc_value: Exception | None, traceback: Any | None
    ) -> bool:
        self.restore_original()
        return False

    def enter(self) -> None:
        pass


@dataclass
class Production(Environment):
    def enter(self) -> None:
        set_aslr(2)

        if get_cpu_vendor() == "intel":
            try:
                set_intel_boost(True)
            except ProgramError:
                pass

        set_swaps(True)

        for cpu in get_cpus("present"):
            cpu.enabled = True
            cpu.governor = "performance"
            cpu.max_freq = cpu.max_hw_freq
            cpu.min_freq = max(cpu.min_hw_freq, 1000000)


@dataclass
class Lightweight(Environment):
    pass


@dataclass
class Lab(Environment):
    def enter(self) -> None:
        set_aslr(0)

        if get_cpu_vendor() == "intel":
            try:
                set_intel_boost(False)
            except ProgramError:
                pass

        set_swaps(False)
        set_drop_caches(3)

        online_cpus = get_cpus("online")

        for cpu in online_cpus:
            if cpu.hyperthread:
                cpu.enabled = False

        online_cpus = get_cpus("online")

        for cpu in online_cpus:
            if cpu.value > 3:
                cpu.enabled = False

        for cpu in get_cpus("online"):
            cpu.governor = "powersave"
            cpu.max_freq = cpu.min_hw_freq
            cpu.min_freq = cpu.min_hw_freq


def main():
    parser = argparse.ArgumentParser(
        description="Manage system environment configurations for energy benchmarking",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s enter lab  # Enter lab environment
  %(prog)s enter prod # Enter production environment
  %(prog)s restore    # Restore original settings
        """,
    )

    subparsers = parser.add_subparsers(dest="command", required=True)

    enter_parser = subparsers.add_parser(
        "enter", help="Enter an environment configuration"
    )
    enter_parser.add_argument(
        "environment",
        choices=["prod", "light", "lab"],
        help="Environment to enter",
    )

    restore_parser = subparsers.add_parser(
        "restore", help="Restore original system configuration"
    )

    args = parser.parse_args()

    env_map = {
        "prod": Production,
        "light": Lightweight,
        "lab": Lab,
    }

    try:
        if args.command == "enter":
            if os.path.exists(BACKUP_FILE):
                print(f"Warning: Backup file already exists. Overwriting.")

            env_class = env_map[args.environment]
            env = env_class()
            print(f"Recording original configuration to {BACKUP_FILE}...")
            env.record_original()
            print(f"Entering {args.environment} environment...")
            env.enter()
            print(f"{args.environment.capitalize()} environment active.")
            print(f"Run '{sys.argv[0]} restore' to revert changes.")

        elif args.command == "restore":
            if not os.path.exists(BACKUP_FILE):
                print(f"Error: No backup file found at {BACKUP_FILE}", file=sys.stderr)
                sys.exit(1)

            print(f"Restoring original configuration from {BACKUP_FILE}...")
            env = Environment()
            env.restore_original()
            os.remove(BACKUP_FILE)
            print("Original configuration restored.")

    except ProgramError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)
    except KeyboardInterrupt:
        print("\nInterrupted", file=sys.stderr)
        sys.exit(130)
    except Exception as e:
        print(f"Unexpected error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()

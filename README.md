# green-languages

`green-languages` is a CLI tool that measures the energy use (in Joules) and other performance metrics of whole programs or individual code sections across different programming languages. It provides start/end markers to wrap the code sections you want to measure.

`green-languages` is also capable to keeping track of multiple repeated internal runs in a loop to detect performance changes during program runtime.

## Usage

Provide a scenario and enable a performance metric bundle:

```sh
green-languages fibonacci.yml --runs 5 --rapl
```

If the program builds and executes successfully `measurements.csv` is created:

```csv
scenario,language,test,nice,affinity,mode,run,internal_run,time,pkg,cores,gpu,ram,psys,cycles,l1d_misses,l1i_misses,llc_misses,branch_misses,c1_core_residency,c6_core_residency,c7_core_residency,c2_pkg_residency,c3_pkg_residency,c6_pkg_residency,c8_pkg_residency,c10_pkg_residency,ended
fibonacci,c,1,,,process,1,1,55321,,,,,,,,,,,,,,,,,,,1772538726074754
fibonacci,c,1,,,process,1,1,51896,1.21,0.815,0.002,,2.134,,,,,,,,,,,,,,1772538757758410
fibonacci,c,1,,,process,2,1,58293,1.237,0.789,0.002,,2.266,,,,,,,,,,,,,,1772538757817866
fibonacci,c,1,,,process,3,1,53568,1.099,0.756,0.004,,2.025,,,,,,,,,,,,,,1772538757872790
fibonacci,c,1,,,process,4,1,56539,0.771,0.521,0.001,,1.632,,,,,,,,,,,,,,1772538757930733
fibonacci,c,1,,,process,5,1,55524,1.073,0.74,0.004,,2.037,,,,,,,,,,,,,,1772538757987432
```

## Available Performance Metric Bundles

| Bundle      | Metric                                     | Unit   | Scope       |
| ----------- | ------------------------------------------ | ------ | ----------- |
|             | Wall-Clock Time (Always Enabled)           | Micros | System-Wide |
| `--rapl`    | RAPL Energy Domains                        | Joules | System-Wide |
| `--cycles`  | CPU Cycles                                 | Count  | Process     |
| `--misses`  | L1D, L1I, LLC Loads Misses & Branch Misses | Count  | Process     |
| `--cstates` | CPU Low Power C-States (CPU Idling)        | Micros | System-Wide |

## Scenarios

A scenario is a **YAML file** holding data to execute a program written in a programming language. A basic example of a scenario written in C is shown. Per default, it measures the performance metrics of the whole process.

```yml
name: fibonacci
language: c
code: |
    #include <stdio.h>

    int fib(int n) {
        if (n <= 1) return n;
        return fib(n - 1) + fib(n - 2);
    }

    int main() {
        int result = fib(35);
        printf("%d\n", result);
        return 0;
    }
```

To measure a code segment in `C`, use `measure_start()` and `measure_end()` from `green.h` and enable `libgreen: true`.

The `libgreen: true` mapping enables the `internal` mode. In this mode, your executed benchmark receives`<internal_runs> <metrics>` as appended command-line arguments.

Pass parameters `measure_start(metrics)` and `measure_stop(context)`:

```yml
name: fibonacci
language: c
libgreen: true
code: |
    #include <stdio.h>
    #include <stdlib.h>
    #include <green.h>

    int fib(int n) {
        if (n <= 1) return n;
        return fib(n - 1) + fib(n - 2);
    }

    int main(int argc, char **argv) {
        if (argc < 3) {
            fprintf(stderr, "Usage: %s <internal_runs> <metrics>\n", argv[0]);
            return 1;
        }
        int internal_runs = atoi(argv[1]);
        const char *metrics = argv[2];

        void *context = measure_start(metrics);
        int result = fib(35);
        printf("%d\n", result);
        measure_stop(context);

        return 0;
    }
```

To measure a code segment multiple times in `C` use `-i, --internal-runs` with an internal loop. This will measure `fib(35)` multiple times within the same process. 

```yml
name: fibonacci
language: c
libgreen: true
code: |
    #include <stdio.h>
    #include <stdlib.h>
    #include <green.h>

    int fib(int n) {
        if (n <= 1) return n;
        return fib(n - 1) + fib(n - 2);
    }

    int main(int argc, char **argv) {
        if (argc < 3) {
            fprintf(stderr, "Usage: %s <internal_runs> <metrics>\n", argv[0]);
            return 1;
        }
        int internal_runs = atoi(argv[1]);
        const char *metrics = argv[2];

        for (int i = 0; i < internal_runs; i++)
        {
            void *context = measure_start(metrics);
            int result = fib(35);
            printf("%d\n", result);
            measure_stop(context);
        }

        return 0;
    }
```

## Supported Languages

### C/C++

```c
#include <stdlib.h>
#include <green.h>

int main(int argc, char **argv) {
    int internal_runs = atoi(argv[1]);
    const char *metrics = argv[2];

    for (int i = 0; i < internal_runs; i++)
    {
        void *context = measure_start(metrics);
        // Code segment to measure
        measure_stop(context);
    }
}
```

### Java

```java
public static void main(final String[] args) {
    int internal_runs = Integer.parseInt(args[0]);
    String metrics = args[1];

    Green green = new Green();

    for (int i = 0; i < internal_runs; i++) {
        long context = green.measureStart(metrics);
        // Code segment to measure
        green.measureStop(context);
    }
}
```

### C#

```c#
using System.Runtime.InteropServices;

class Program {
    [DllImport("libgreen", EntryPoint = "measure_start")]
    public static extern IntPtr measure_start([MarshalAs(UnmanagedType.LPStr)] string metrics);

    [DllImport("libgreen", EntryPoint = "measure_stop")]
    public static extern void measure_stop(IntPtr context);

    public static void Main(string[] args) {
        int internal_runs = int.Parse(args[0]);
        string metrics = args[1];

        for (int i = 0; i < internal_runs; i++) {
            IntPtr context = measure_start(metrics);
            // Code segment to measure
            measure_stop(context);
        }
    }
}
```

### Rust

```rust
    use std::ffi::CString;

    #[link(name = "green")]
    unsafe extern "C" {
        fn measure_start(metrics: *const std::ffi::c_char) -> *mut std::ffi::c_void;
        fn measure_stop(context: *mut std::ffi::c_void);
    }

    fn main() {
        let internal_runs: usize = args[0].parse().expect("Invalid integer");
        let metrics = CString::new(args[1].as_str()).expect("Invalid metrics string");

        for _ in 0..internal_runs {
            unsafe {
                let context = measure_start(metrics.as_ptr());
                // Code segment to measure
                measure_stop(context);
            }
        }
    }
```

### Ruby

```ruby
# Coming soon
```

### Python

```python
# Coming soon
```

## Tests

To define multiple tests within for same scenario, enumerate the tests at the end of the file using `---` as delimiter. The example also shows how to pass compile time arguments and input, and how to verify if the output of the program is correct. 

> [!IMPORTANT]
> A current limitation is that `expected_stdout` must be in `base64` format:

```yml
name: fibonacci
language: c
libgreen: true
code: |
    #include <stdio.h>
    #include <stdlib.h>
    #include <green.h>

    int fib(int n) {
        if (n <= 1) return n;
        return fib(n - 1) + fib(n - 2);
    }

    int main(int argc, char **argv) {
        if (argc < 4) {
            fprintf(stderr, "Usage: %s <n> <internal_runs> <metrics>\n", argv[0]);
            return 1;
        }

        int n = atoi(argv[1]);
        int internal_runs = atoi(argv[2]);
        const char *metrics = argv[3];

        for (int i = 0; i < internal_runs; i++)
        {
            void *context = measure_start(metrics);
            int result = fib(n);
            printf("%d\n", result);
            measure_stop(context);
        }

        return 0;
    }
compile_options: [-O3, -march=native]
---
name: optimized 30
arguments: [30]
expected_stdout: !!binary |
    ODMyMDQwCg==
---
name: optimized 35
arguments: [35]
expected_stdout: !!binary |
    OTIyNzQ2NQo=
---
name: unoptimized 40
arguments: [40]
compile_options: []
expected_stdout: !!binary |
    MTAyMzM0MTU1Cg==
```

## All Scenario Mappings

| Field           | Type   | Required | Description                                                                                 |
| --------------- | ------ | -------- | ------------------------------------------------------------------------------------------- |
| name            | String | Yes      | Scenario identifier used in output and logs. Test-level defines the test name               |
| language        | String | Yes      | Language: `c`, `cpp`, `cs`, `java`, `rust`, `python`,                                       |
| code            | String | Yes      | Source code to compile and execute                                                          |
| description     | String | No       | Human-readable scenario description                                                         |
| compile_options | List   | No       | Compiler flags for compiled languages. Test-level overrides scenario-level                  |
| runtime_options | List   | No       | Runtime interpreter flags for non-compiled languages. Test-level overrides scenario-level   |
| arguments       | List   | No       | Program arguments. Test-level overrides scenario-level. Supports strings, numbers, booleans |
| framework       | String | No       | Required for C#: target framework (e.g.,net8.0)                                             |
| dependencies    | List   | No       | External package dependencies with name and optional version                                |
| affinity        | List   | No       | CPU core pinning (e.g.,[0, 1] restricts to cores 0-1). Test-level overrides scenario-level  |
| nice            | Int    | No       | Process priority (-20 highest, 19 lowest). Test-level overrides scenario-level              |
| stdin           | Binary | No       | Base64-encoded input piped to program. Test-level overrides scenario-level                  |
| expected_stdout | Binary | No       | Base64-encoded expected output for verification. Test-level overrides scenario-level        |


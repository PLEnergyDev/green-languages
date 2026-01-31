# green-languages

`green-languages` is a CLI tool that measures the energy use (in Joules) and other performance metrics of whole programs or individual code sections across different programming languages. It provides start/end markers to wrap the code sections you want to measure. These markers are also used to track a measurement iteration count, allowing repeated measurements for the same code section within a loop to detect performance changes during program runtime.

## Usage

Provide a scenario and enable a performance metric:

```sh
green-languages fibonacci.yml --rapl -i5
```

If the program builds and runs successfully, the `measurements` dir is created, which contains `measurements.csv`, `measurements.log` and the build artifacts. Contents of `measurements.csv`:

```csv
scenario,language,test,mode,iteration,time,pkg,cores,gpu,dram,psys,ended
fibonacci,c,1,process,1,56288,1.087,0.793,0.0,,1.84,1762346358725229
fibonacci,c,1,process,2,57025,0.905,0.667,0.0,,1.581,1762346358786143
fibonacci,c,1,process,3,58306,0.997,0.753,0.0,,1.692,1762346358848497
fibonacci,c,1,process,4,56242,0.654,0.421,0.0,,1.266,1762346358909304
fibonacci,c,1,process,5,53974,0.83,0.606,0.0,,1.456,1762346358967603
```

## Available Performance Metrics

| Flag        | Metric                                     | Unit   | Scope       |
| ----------- | ------------------------------------------ | ------ | ----------- |
|             | Wall-Clock Time (Always Enabled)           | Micros | System-Wide |
| `--rapl`    | RAPL Energy Domains                        | Joules | System-Wide |
| `--cycles`  | CPU Cycles                                 | Count  | Process     |
| `--misses`  | L1D, L1I, LLC Loads Misses & Branch Misses | Count  | Process     |
| `--cstates` | CPU Low Power C-States (CPU Idling)        | Micros | System-Wide |

## Scenarios

A scenario is a **YML file** holding data to execute a program written in a programming language. A basic example of a scenario written in C is shown. Per default, it measures the performance metrics of the whole process.

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

To measure a code segment in C, use `start_gl()` and `end_gl()` from the `signals.h` library and set `mode: external`.

```yml
name: fibonacci
language: c
code: |
    #include <stdio.h>
    #include <signals.h>

    int fib(int n) {
        if (n <= 1) return n;
        return fib(n - 1) + fib(n - 2);
    }

    int main() {
        start_gl();
        int result = fib(35);
        end_gl();
        printf("%d\n", result);
        return 0;
    }
mode: external
```

To measure a code segment in C multiple times, put the markers in an loop and set `mode: internal`. Then use `green-languages fibonacci.yml --rapl -i10` with `-i, --iterations` and the iteration count. This will measure `printf("Hello, World!");` 10 times within the same process.

```yml
name: fibonacci
language: c
code: |
    #include <stdio.h>
    #include <signals.h>

    int fib(int n) {
        if (n <= 1) return n;
        return fib(n - 1) + fib(n - 2);
    }

    int main() {
        while (1) {
            if (start_gl() == 0) break;
            int result = fib(35);
            end_gl();
            printf("%d\n", result);
        }
        return 0;
    }
mode: internal
```

## Supported Programming Languages

### C/C++

```c
#include <signals.h>

int main() {
    while (1) {
        if (start_gl() == 0) break;
        // Code segment to measure
        end_gl();
    }
}
```

### Java

```java
public static void main(final String[] args) throws Exception {
    Signals signals = new Signals();

    while (true) {
        if (signals.startGl() == 0) break;
        // Code segment to measure
        signals.endGl();
    }
}
```

### C#

```c#
using System.Runtime.InteropServices;

class Program {
    [DllImport("libsignals", EntryPoint = "start_gl")]
    private static extern bool start_gl();

    [DllImport("libsignals", EntryPoint = "end_gl")]
    private static extern void end_gl();

    public static void Main(string[] args) {
        while (true) {
            if (!start_gl()) break;
            // Code segment to measure
            end_gl();
        }
    }
}
```

### Rust

```rust
// Coming soon
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

To define multiple tests within for same scenario, enumerate the tests at the end of the file using `---` as delimiter. Example also shows how to pass compile time arguments, input and verify if the output of the code is correct. 

> [!IMPORTANT]
> A current limitation is that `expected_stdout` needs to be in `base64` format:

```yml
name: fibonacci
language: c
code: |
    #include <stdio.h>
    #include <stdlib.h>
    #include <signals.h>

    int fib(int n) {
        if (n <= 1) return n;
        return fib(n - 1) + fib(n - 2);
    }

    int main(int argc, char **argv) {
        int n = argc > 1 ? atoi(argv[1]) : 35;
        
        while (1) {
            if (start_gl() == 0) break;
            int result = fib(n);
            end_gl();
            printf("%d\n", result);
        }
        return 0;
    }
mode: internal
compile_options: [-O3, -march=native]
---
name: 30 O3
arguments: [30]
expected_stdout: !!binary |
    ODMyMDQwCg==
---
name: 35 O3
arguments: [35]
expected_stdout: !!binary |
    OTIyNzQ2NQo=
---
name: 40
arguments: [40]
compile_options: []
expected_stdout: !!binary |
    MTAyMzM0MTU1Cg==
```

## All Scenario Fields

| Field            | Type    | Required | Description                                                                                                                                              |
| ---------------- | ------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| name             | String  | Yes      | Scenario identifier used in output and logs                                                                                                              |
| language         | String  | Yes      | Programming language: `c`, `cpp`, `cs`, `java`, `rust`, `python`,                                                                                        |
| code             | String  | Yes      | Source code to compile and execute                                                                                                                       |
| description      | String  | No       | Human-readable scenario description                                                                                                                      |
| mode             | String  | No       | Measurement strategy: `process` (default, entire program), `external` (single iteration within process), `internal` (multiple iterations within process) |
| compile_options  | List    | No       | Compiler flags for compiled languages. Test-level overrides scenario-level                                                                               |
| runtime_options  | List    | No       | Runtime interpreter flags for non-compiled languages. Test-level overrides scenario-level                                                                |
| arguments        | List    | No       | Program arguments. Test-level overrides scenario-level. Supports strings, numbers, booleans                                                              |
| framework        | String  | No       | Required for C#: target framework (e.g.,net8.0)                                                                                                          |
| dependencies     | List    | No       | External package dependencies with name and optional version                                                                                             |
| affinity         | List    | No       | CPU core pinning (e.g.,[0, 1] restricts to cores 0-1). Test-level overrides scenario-level                                                               |
| niceness         | Integer | No       | Process priority (-20 highest, 19 lowest). Test-level overrides scenario-level                                                                           |
| stdin            | Binary  | No       | Base64-encoded input piped to program. Test-level overrides scenario-level                                                                               |
| expected_stdout  | Binary  | No       | Base64-encoded expected output for verification. Test-level overrides scenario-level                                                                     |


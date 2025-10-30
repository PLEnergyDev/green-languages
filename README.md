# green-languages

`green-languages` (`gl`) is a CLI tool that measures energy consumption and other performance metrics of whole programs or individual lines of code across different programming languages. It provides start/end measurement markers to wrap code sections. These markers are also used to track the measurement count, allowing repeated measurements for the same code section within a loop to detect performance changes during program runtime.

## Usage

Provide a scenario and enable a performance metric to the `gl` CLI tool:

```sh
gl fibonacci.yml --rapl-pkg --rapl-cores --time
```

Upon success, the `results` directory is created containing `results.csv`, `gl.log` and build artifacts:

```csv
scenario,language,test,mode,iteration,time,pkg,cores,gpu,dram,psys,cycles,cache_misses,branch_misses,ended
fibonacci,c,30 O3,internal,1,1317834,0.0513916015625,0.041259765625,,,,,,,1761828780087218
fibonacci,c,35 O3,internal,1,11007968,0.365234375,0.283935546875,,,,,,,1761828780157678
fibonacci,c,40,internal,1,545718361,13.006591796875,10.04339599609375,,,,,,,1761828780731557
```

## Available Performance Metrics

| Flag                 | Metric                | Unit        | Description                               |
| -------------------- | --------------------- | ----------- | ----------------------------------------- |
| `--time`             | Execution time        | Nanoseconds | Wall-clock execution duration             |
| `--rapl-pkg`         | Package energy        | Joules      | CPU package power consumption             |
| `--rapl-cores`       | CPU cores energy      | Joules      | CPU cores power consumption               |
| `--rapl-gpu`         | GPU energy            | Joules      | Integrated GPU power consumption          |
| `--rapl-dram`        | DRAM energy           | Joules      | Memory power consumption                  |
| `--rapl-psys`        | Platform energy       | Joules      | Total platform power consumption          |
| `--rapl-all`         | All RAPL domains      |             | Enables all available RAPL domains        |
| `--hw-cycles`        | CPU cycles            | Count       | Total CPU cycles executed                 |
| `--hw-cache-misses`  | Cache misses          | Count       | Last-level cache miss count               |
| `--hw-branch-misses` | Branch misses         | Count       | Branch misprediction count                |
| `--hw-all`           | All hardware counters |             | Enables all hardware performance counters |

## Scenarios

A scenario is a **YML file** holding data to execute a program written in a programming language. A basic example of a scenario written in C is shown. Per default, it measures the performance metrics of the whole program.

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

To measure individual lines of code in C, use `start_measurement()` and `end_measurement()` from the `measurement.h` library and enable the `measurement_mode: external` flag.

```yml
name: fibonacci
language: c
code: |
    #include <stdio.h>
    #include <measurements.h>

    int fib(int n) {
        if (n <= 1) return n;
        return fib(n - 1) + fib(n - 2);
    }

    int main() {
        start_measurement();
        int result = fib(35);
        end_measurement();
        printf("%d\n", result);
        return 0;
    }
measurement_mode: external

```

To measure individual lines of code in C multiple times, put the markers in an loop and enable the `measurement_mode: internal` flag. Then use `gl <scenario_filename.yml> --rapl-pkg --iterations 10` with `-i, --iterations` and the iteration count. This will measure `printf("Hello, World!");` 10 times within the same process.

```yml
name: fibonacci
language: c
code: |
    #include <stdio.h>
    #include <measurements.h>

    int fib(int n) {
        if (n <= 1) return n;
        return fib(n - 1) + fib(n - 2);
    }

    int main() {
        while (1) {
            if (start_measurement() == 0) break;
            int result = fib(35);
            end_measurement();
            printf("%d\n", result);
        }
        return 0;
    }
measurement_mode: internal

```

## Supported Programming Languages

### C/C++

```c
#include <measurements.h>

int main() {
    while (1) {
        if (start_measurement() == 0) break;
        // Code segment to measure
        end_measurement();
    }
}
```

### Java

```java
public static void main(final String[] args) throws Exception {
    Measurements measurements = new Measurements();

    while (true) {
        if (measurements.startMeasurement() == 0) break;
        // Code segment to measure
        measurements.endMeasurement();
    }
}
```

### C#

```c#
using System.Runtime.InteropServices;

class Program {
    [DllImport("libmeasurements", EntryPoint = "start_measurement")]
    private static extern bool start_measurement();

    [DllImport("libmeasurements", EntryPoint = "end_measurement")]
    private static extern void end_measurement();

    public static void Main(string[] args) {
        while (true) {
            if (!start_measurement()) break;
            // Code segment to measure
            end_measurement();
        }
    }
}
```

### Rust

```rust
// Add Rust implementation
```

### Ruby

```ruby
# Add Ruby implementation
```

### Python

```python
# Add Python implementation
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
    #include <measurements.h>

    int fib(int n) {
        if (n <= 1) return n;
        return fib(n - 1) + fib(n - 2);
    }

    int main(int argc, char **argv) {
        int n = argc > 1 ? atoi(argv[1]) : 35;
        
        while (1) {
            if (start_measurement() == 0) break;
            int result = fib(n);
            end_measurement();
            printf("%d\n", result);
        }
        return 0;
    }
measurement_mode: internal
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

| Field             |  Type     |  Required  |  Description                                                                                                                                             
| ----------------- | --------- | ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| name              | String    | Yes        | Scenario identifier used in output and logs                                                                                                               |
| language          | String    | Yes        | Programming language: `c`, `cpp`, `cs`, `java`, `rust`, `python`,                                                                                         |
| code              | String    | Yes        | Source code to compile and execute                                                                                                                        |
| description       | String    | No         | Human-readable scenario description                                                                                                                       |
| measurement_mode  | String    | No         | Measurement strategy: `process` (default, entire program), `external` (single iteration within process), `internal` (multiple iterations within process)  |
| compile_options   | List      | No         | Compiler flags for compiled languages. Test-level overrides scenario-level                                                                                |
| runtime_options   | List      | No         | Runtime interpreter flags for non-compiled languages. Test-level overrides scenario-level                                                                 |
| arguments         | List      | No         | Program arguments. Test-level overrides scenario-level. Supports strings, numbers, booleans                                                               |
| framework         | String    | No         | Required for C#: target framework (e.g.,net8.0)                                                                                                           |
| packages          | List      | No         | External package dependencies with name and optional version                                                                                              |
| dependencies      | List      | No         | System dependencies with name and optional version                                                                                                        |
| affinity          | List      | No         | CPU core pinning (e.g.,[0, 1] restricts to cores 0-1). Test-level overrides scenario-level                                                                |
| niceness          | Integer   | No         | Process priority (-20 highest, 19 lowest). Test-level overrides scenario-level                                                                            |
| stdin             | Binary    | No         | Base64-encoded input piped to program. Test-level overrides scenario-level                                                                                |
| expected_stdout   | Binary    | No         | Base64-encoded expected output for verification. Test-level overrides scenario-level                                                                      |

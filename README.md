# green-languages

`green-languages` (`gl`) is a CLI tool that measures energy consumption and other performance metrics of whole programs or individual lines of code across different programming languages. It provides start/end measurement markers to wrap code sections. These markers are also used to track the measurement count, allowing repeated measurements for the same code section within a loop to detect performance changes during program runtime.

## Usage

Provide a scenario and enable a performance metric to the `gl` CLI tool:

```sh
gl fib.yml --rapl --cycles --branch-misses --cache-misses --cstates -i5
```

Upon success, the `results` dir is created containing `results.csv`, `gl.log` and build artifacts. Contents of `results.csv`:

```csv
scenario,language,test,mode,iteration,time,pkg,cores,gpu,dram,psys,cycles,l1d_misses,l1i_misses,llc_misses,branch_misses,c1_core_residency,c3_core_residency,c6_core_residency,c7_core_residency,c2_pkg_residency,c3_pkg_residency,c6_pkg_residency,c8_pkg_residency,c10_pkg_residency,ended
fibonacci,c,1,process,1,56288,1.087,0.793,0.0,,1.84,14850497,665,1613,27,342,731518242,,1614601814,876914016,0,0,0,0,0,1762346358725229
fibonacci,c,1,process,2,57025,0.905,0.667,0.0,,1.581,16596192,268,1274,3,339,420737148,,1701854796,1236024600,0,0,0,0,0,1762346358786143
fibonacci,c,1,process,3,58306,0.997,0.753,0.0,,1.692,9990690,280,1358,0,345,303858282,,1724073718,1367192966,0,0,0,0,0,1762346358848497
fibonacci,c,1,process,4,56242,0.654,0.421,0.0,,1.266,9598776,268,1258,1,348,207767508,,1578030626,1575898272,0,0,0,0,0,1762346358909304
fibonacci,c,1,process,5,53974,0.83,0.606,0.0,,1.456,12048084,277,1372,0,343,266517498,,1540066812,1350901600,0,0,0,0,0,1762346358967603

```

## Available Performance Metrics

| Flag              | Metric                       | Unit              |
| ----------------- | ---------------------------- | ----------------- |
| `--rapl`          | All RAPL energy domains      | Joules            |
| `--cycles`        | CPU cycles & wall-clock time | Count/Miroseconds |
| `--cache-misses`  | L1d, L1i, LLC loads misses   | Count             |
| `--branch-misses` | Branch misses                | Count             |
| `--cstates`       | All CPU low power C-states   | Count             |

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

To measure individual lines of code in C multiple times, put the markers in an loop and enable the `measurement_mode: internal` flag. Then use `gl fib.yml --rapl -i10` with `-i, --iterations` and the iteration count. This will measure `printf("Hello, World!");` 10 times within the same process.

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

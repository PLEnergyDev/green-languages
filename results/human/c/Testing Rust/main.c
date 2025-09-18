#include <stdio.h>
#include <stdlib.h>

void run_benchmark(int argc, char **argv) {
    int m = atoi(argv[1]);
    double sum = 0.0;
    int n = 0;
    while (sum < m) {
        n++;
        sum += 1.0 / n;
    }
    printf("%d\n", n);
}

int main(int argc, char **argv) {
    run_benchmark(argc, argv);
    return 0;
}

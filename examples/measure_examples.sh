#!/bin/bash

gl ./*/*.yml --iterations 10 --sleep 1 --rapl-all --hw-all --time -o example_results.csv

#!/bin/bash
green-languages \
    {internal,process}/{c,cs,java,rust}/{division-loop,matrix-multiplication,polynomial-evaluation}.yml \
    --rapl --misses --cycles --cstates \
    --internal-runs 5 --cooldown 5000
    --output "obtained-measurements"


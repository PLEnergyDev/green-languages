#!/bin/bash
green-languages ./process/{c,cs,java,rust}/{division-loop,matrix-multiplication,polynomial-evaluation}.yml \
    --rapl --misses --cycles --cstates \
    --cooldown 5000 # 5 seconds
    --output "obtained-measurements"

green-languages ./process/{c,cs,java,rust}/{division-loop,matrix-multiplication,polynomial-evaluation}.yml \
    --rapl --misses --cycles --cstates \
    --cooldown 5000 # 5 seconds
    --niceness -20 --affinity 5
    --output "obtained-measurements"

green-languages ./internal/{c,cs,java,rust}/{division-loop,matrix-multiplication,polynomial-evaluation}.yml \
    --rapl --misses --cycles --cstates \
    --cooldown 5000 # 5 seconds
    --internal-runs 5
    --output "obtained-measurements"


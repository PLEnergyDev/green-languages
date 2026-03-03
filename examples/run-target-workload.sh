#!/bin/bash
green-languages \
    {c,cs,java,rust}/{division-loop,matrix-multiplication,polynomial-evaluation}.yml \
    --rapl --misses --cycles --cstates \
    --runs 10 --cooldown 5000


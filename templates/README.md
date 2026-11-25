# Dialect templates

Data files for the Presto, Doris and Druid dialects: templates taken from Cube at the pinned commit, the interval units of each engine, and the query features an engine cannot answer.

A template under `kernel/` is not Cube's: the adapter reads it. `kernel/rolling_select` is the rolling window of an engine that joins on equality only; a dialect that has it gets the planner's rolling SELECT in that form.

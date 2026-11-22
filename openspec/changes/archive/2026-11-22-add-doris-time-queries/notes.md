# Notes

Using change add-doris-time-queries. Owner authorized next design and development.
B0 only; real local Doris Docker is mandatory. No owner decision is open.

Implemented Doris DATE_TRUNC, CONVERT_TZ and DATETIMEV2(3) UTC-bound casts.
The native prepared statement path executed 13 freshly compiled requests on real
local Docker Doris 4.1.3, including Tokyo millisecond bounds and New York DST.
All passed; the owned container was removed. Eleven new SQL goldens reviewed;
no previous golden changed. cargo insta review reports no pending snapshots.
Remaining: final checks, publication and cross-vendor delivery. No blocker.
Exact-head evidence belongs in PR comments, not a follow-up evidence commit.

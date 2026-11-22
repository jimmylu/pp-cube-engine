# Spec Delta

## ADDED Requirements

### Requirement: Doris temporal execution tier
The Doris execution tier SHALL execute the live-compiled temporal cases of the execution manifest on a real Doris engine and fail on a bucket, boundary or security regression. This SHALL NOT imply full B0 execution acceptance.
#### Scenario: B0-DORIS-010 Temporal execution smoke (T2)
- **WHEN** compiled temporal queries execute on a real local Doris fixture
- **THEN** calendar buckets, millisecond boundary inclusion and security restrictions equal independently chosen expected results

# Spec Delta

## ADDED Requirements

### Requirement: Benchmark for any engine
The repository SHALL hold an integration test kit that loads one benchmark into Presto, Doris or Druid and runs one suite of questions against it. The benchmark's rows SHALL be those of the property-insurance fixture, and what the suite expects of each dialect, an answer or a refusal of a kind, SHALL be what the kernel does for that dialect.
#### Scenario: B1-KIT-001 The benchmark and the suite follow the kernel
- **WHEN** the kit's data is rendered as Presto statements and each question of the suite is compiled for Presto, Doris and Druid under its user's rules
- **THEN** the statements are the fixture's, each question compiles with the stated columns or is refused with the stated kind, and the guide states the resulting counts for each engine

### Requirement: Guide and kit agree
The integration test guide SHALL name only commands and options the kit has, SHALL show every command of the kit, and the kit's example configuration SHALL show every setting the guide uses.
#### Scenario: B1-KIT-002 The guide's commands exist
- **WHEN** every command line of the kit in the guide and in the kit's README is read
- **THEN** each command and each option is one the kit registers, every command of the kit appears, and an unknown command or option fails the check

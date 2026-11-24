# Spec Delta

## ADDED Requirements

### Requirement: Views in the service
Compile SHALL accept queries on the views of a published version and return the facade result. GetMeta SHALL list each public view after the cubes, as a cube without joins or segments: its public members under their names in the view, each measure with the type of the measure it takes.
#### Scenario: B1-API-001 View compilation and metadata
- **WHEN** a version with a view is published, a query on the view is compiled, and metadata is requested
- **THEN** the compilation equals the facade's, and the view is listed last with its title, its data source, the public members it takes and the type of the measure behind each

# Spec Delta

## ADDED Requirements

### Requirement: Model directories
A model MAY be given as a directory of files, one cube per file under `cubes/` and one view per file under `views/`, each file named after what it defines in snake_case with the extension `.yml` or `.yaml`. Assembling them SHALL give the model one document with the same content gives, whatever order the files arrive in. A file that is misplaced, misnamed, not of the contract, or holds more or less than one definition SHALL fail, naming the file; a directory without a cube is not a model.
#### Scenario: B1-MODEL-004 Directory of files
- **WHEN** two cube files and a view file are assembled in either order, and directories hold a misplaced file, a file whose name differs from its cube, a file with two cubes, a view under `cubes/`, a syntax fault or no cube
- **THEN** the first two give the same valid model as the single document, and each of the others fails naming the file, with the position of a syntax fault

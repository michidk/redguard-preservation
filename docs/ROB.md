# ROB File Format Documentation

There are 31 ROB files, found in the `fxart` directory.
They hold multiple 3D meshes.

## Format

The file is a binary file (little endian) composed of a header and a list of meshes.

### Header

| Adress | Size | Data Type (Rust) | Description |
|--------|------|-----------|-------------|
| 0x00   | 4    |      | Magic number: `OARC` |

### Mesh

| Adress | Size | Description |

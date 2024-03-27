# Spec Generator

CLI utility for generating a JSON version of the API's OpenAPI spec.

## Usage

From the repository root, run the following command:

```sh
$ cargo run --package spec-generator
```

To generate the spec, run:

```sh
$ cargo run --package spec-generator > api-spec.json
```

To validate an existing spec, run:

```sh
$ cargo run --package spec-generator -- --check api-spec.json
```

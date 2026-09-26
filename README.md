# Demo picker

A small photo picker against the Immich v1.116.2 search API. It runs
random, smart, and metadata search through one shared response parser
and collects asset IDs for the picker.

## Configure

Set `IMMICH_BASE_URL` to the server address.

```bash
export IMMICH_BASE_URL=http://localhost:2283/api
```

## Run

Print one random page of asset IDs.

```bash
cargo run -- --base-url http://localhost:2283/api
```

Omit the flag to use `IMMICH_BASE_URL` instead.

## Test

```bash
cargo nextest run
```

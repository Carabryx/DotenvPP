# dotenvpp-cli

Command-line interface for DotenvPP.

Current commands:

- `dotenvpp check` to validate a file or layered environment stack
- `dotenvpp run` to load values and execute a command
- `--file` to target one explicit `.env` file
- `--env` / `-e` to load layered files like `.env.production` and `.env.production.local`

Library users should depend on the top-level [`dotenvpp`](../../README.md) crate.

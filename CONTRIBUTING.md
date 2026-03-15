# Contributing

Thanks for contributing to `model-meter`.

`model-meter` is intended to stay model-agnostic. Contributions should improve the shared usage-meter experience or add provider support in a way that keeps support levels honest.

## Before You Start

- use issues for bugs, feature requests, and usage problems
- keep changes focused
- avoid bundling unrelated changes into one pull request

If you are unsure whether a change fits the project, open an issue first.

## How To Contribute

This repository uses the normal open-source fork model.

Please do not work directly on the main repository branch structure unless you are explicitly maintaining the repo.

The expected flow is:

1. fork the repository
2. create a branch in your fork
3. make your change
4. open a pull request back to `main`

## Development

Build:

```bash
cargo build
```

Test:

```bash
cargo test
```

Run locally:

```bash
cargo run -- codex
```

`codex` is the first working provider command, not the whole product.

## Pull Request Guidelines

- describe what changed and why
- include any user-facing behavior changes clearly
- update docs when commands or behavior change
- add tests when practical
- keep PRs small enough to review quickly

## Project Direction

The project is intentionally strict about trustworthiness.

That means:

- prefer official or clearly-supported data sources
- do not present unsupported usage as authoritative
- call out partial support explicitly
- keep the product model-agnostic even when one provider is ahead of the others

## Reporting Problems

If the tool does not work for you, open an issue with:

- your OS
- how you installed the tool
- the command you ran
- the output or error you saw
- whether you were logged into Codex already

## Feature Requests

Feature requests are welcome.

Good feature requests explain:

- the workflow you are trying to support
- what is missing today
- what command or output you expected
- whether the request depends on an official provider surface
- how the request fits the broader model-agnostic direction of the tool

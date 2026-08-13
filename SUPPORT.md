# Support

Invokrum is currently a pre-release open-source project maintained on a best-effort basis.

## Questions and design discussion

Use GitHub Issues for:

- reproducible bugs;
- scoped feature proposals;
- documentation gaps;
- architecture questions tied to a concrete use case;
- compatibility concerns;
- contribution planning.

Before opening an issue, search existing issues and review the [documentation index](docs/README.md).

## Bug reports

A useful bug report includes:

- the commit or version;
- operating system and filesystem details;
- the smallest relevant pack, profile, and command;
- expected and observed results;
- deterministic reproduction steps;
- redacted diagnostics;
- whether symbolic links, unusual paths, or sensitive variables are involved.

Do not attach proprietary prompt packs or credentials.

## Feature requests

Describe the underlying problem and operational constraints before proposing a specific interface. Explain:

- why the behavior belongs in Invokrum rather than a host adapter or consumer policy pack;
- determinism and compatibility implications;
- security and trust-boundary implications;
- a minimal acceptance test.

## Security issues

Follow [SECURITY.md](SECURITY.md). Do not report vulnerabilities publicly.

## What is not currently offered

- commercial support or service-level agreements;
- private implementation consulting through the issue tracker;
- support for unreleased commands documented as planned;
- review of confidential prompt content;
- runtime support for models, agents, or third-party host integrations.

Support commitments may be revised after stable releases exist.
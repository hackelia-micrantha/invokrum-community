# Security policy

## Reporting a vulnerability

Do not disclose suspected vulnerabilities in a public issue, discussion, pull request, or generated fixture.

Use GitHub's private vulnerability reporting feature for `hackelia-micrantha/invokrum-community` when available. A useful report includes:

- affected commit or released version;
- reproduction steps or proof of concept;
- expected and observed behavior;
- realistic impact and attack path;
- relevant platform/filesystem details;
- whether untrusted packs, publisher identities, installation evidence, or sensitive values are involved;
- suggested remediation, if known.

Never include live credentials, access tokens, private signing keys, or third-party confidential data.

## Repository split boundary

This repository is the public community distribution. Private canonical development history is not a reporting channel and should not be copied into a public report.

After the initial implementation baseline is imported, the public threat model and supported-version policy in this repository define the security claims available to community users. Security claims must map to controls and tests present in the community checkout; private-only controls must not be represented as community guarantees.

## Supported versions

Until the baseline import is complete, this repository contains split scaffolding only and has no independently released implementation version. After cutover, the default branch and explicitly supported public releases receive best-effort security fixes unless an advisory states otherwise.
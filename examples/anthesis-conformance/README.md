# Anthesis reference-consumer conformance

This directory is a **synthetic external-consumer fixture** for Invokrum. It demonstrates that selected prompt-overlay behavior observed in Anthesis can be represented as ordinary `invokrum.dev/v1` pack data without adding Anthesis-specific behavior to Invokrum core.

## Reference revision

The fixture was derived from Anthesis `main` at commit `116ad125f790fdfc592f186b546ac0dad4ffe148`, specifically the overlay class/order/cardinality and compatibility rules documented in `prompts/overlays/manifest.md` and the profile-selection surfaces under `prompts/profiles/`.

Anthesis is proprietary. No Anthesis prompt body, policy corpus, or implementation source is vendored here. All overlay text in this directory is newly written synthetic test content under Invokrum's Apache-2.0 license.

## Behaviors represented

The pack preserves the observed class precedence as data:

`core -> governance -> compliance -> security -> operations -> delivery -> integration -> environment -> mode -> artifact -> lenses -> quality -> cost`

The selected conformance contract exercises:

- two required synthetic core overlays;
- exactly one environment;
- exactly one mode;
- additive governance and quality overlays;
- read-only governance incompatible with implementation mode;
- deterministic class-ordered rendering.

`audit-ci` is the positive golden profile. Its output must match `expected/audit-ci-context.md` byte for byte.

The invalid fixtures demonstrate causal alignment where the two models overlap:

- `invalid/multiple-mode.yaml` fails because mode cardinality is greater than one;
- `invalid/multiple-environment.yaml` fails because environment cardinality is greater than one;
- `invalid-read-only-implementation` fails because the selected read-only governance overlay is explicitly incompatible with implementation mode.

## Intentional differences

This is not a permanent Anthesis compatibility promise and does not attempt to reproduce Anthesis-specific semantics that are outside Invokrum's generic model. In particular:

- Anthesis authority/STOP semantics remain Anthesis policy, not Invokrum engine behavior;
- phase defaults, `use_when`/`do_not_use_when`, profile aliases, runtime topology, preflight checks, and environment-variable validation are not modeled by this pack;
- Anthesis recommends at most one environment in its overlay manifest while its session workflow selects a single environment; this fixture intentionally encodes exactly one environment to match issue #7's bounded conformance requirement;
- this suite does not execute proprietary Anthesis code in Invokrum CI.

If Anthesis changes, update this fixture only after reviewing the new reference revision and documenting any intentional divergence. Removing this directory and its tests must not alter Invokrum core, CLI, host contract, or release behavior.

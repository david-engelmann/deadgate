# deadgate

It reconciles the required status checks in branch protection, the jobs CI actually executes, and the trigger conditions inside each workflow, and prints the required check your team believes guards main next to the last date it actually ran.

## What this is

A required check can enforce nothing for weeks after a rename while the merge button stays green the whole time, and skipped jobs reporting success is a second documented variant of the same break. No CI platform self-tests that a bad pull request still gets refused. Deadgate compares the three sources of truth, protection settings, workflow definitions, and execution history, and names the checks that exist only on paper. The boundary with its siblings is one sentence. Deadgate asks whether it ran, finaltree asks what it ran on, blindgate asks what it read.

## Status

Seeded in October 2026. The build has not started. Part of the impatient programming portfolio, in the Gate Truth arc.

## Licence

MIT. See LICENSE.

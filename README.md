# deadgate

It reconciles the required status checks in branch protection, the jobs CI actually executes, and the trigger conditions inside each workflow, and prints the required check your team believes guards main next to the last date it actually ran.

## What this is

A required check can enforce nothing for weeks after a rename while the merge button stays green the whole time, and skipped jobs reporting success is a second documented variant of the same break. No CI platform self-tests that a bad pull request still gets refused. Deadgate compares the three sources of truth, protection settings, workflow definitions, and execution history, and names the checks that exist only on paper. The boundary with its siblings is one sentence. Deadgate asks whether it ran, finaltree asks what it ran on, blindgate asks what it read.

## Status

Seed is done and the seed commit sits on the local main branch. The contract in CONTRACT.md holds the obligations for this repo, and the seed concept stands as slice 1.

Slice 2 is built on a local branch as the Round 1 work increment. The slice 2 obligations in CONTRACT.md are met by the test suite. Deadgate ingests a branch protection export, a folder of workflow definitions, and an execution history export, all as local files and with no network call at any point, and it reconciles every required status check against the jobs that can produce it for the branch and the records those jobs left behind in the history window. A check that ran to a conclusion reads alive with its last run date, and a check that exists only on paper is named in one of the three paper kinds, no producer, never ran, or skipped only. The rename case reads no producer with the stale date of its last run under the old name standing beside it as the evidence. The two skip shapes are told apart, because a job skipped inside a firing workflow satisfies the requirement without running and reads skipped only, while a path filtered trigger creates no run at all and its check reads never ran with the note standing on it. The report renders as JSON under the format name deadgate-report/1 and as a markdown table from the same facts, deterministic byte for byte on the same inputs, and a committed example under examples shows one check of each kind. AGENTS.md stands in this repo, the canonical template of the program with this repo's annex below its marked line. Pinned formats stand at deadgate-report/1. The Round 1 review fixes recorded in CONTRACT.md tighten the readings the two reviews found loose, so a run now counts toward a check only from the workflow that produces it, a matrix combination counts only in the exact form the matrix generates, branch filters follow the GitHub filter pattern rules in the order they are written, a written filter is never read as no filter, and the path filtered note stands only on a never ran check whose absence the filter explains.

Closed rounds stand at 1 of 100. Round 1 closed after audit and two-axis review and red team and owner approval, with the review fixes landed on the same branch.

Part of the impatient programming portfolio, in the Gate Truth arc.

## Licence

MIT. See LICENSE.

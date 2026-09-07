# Sync GitHub commits to origin

## Objective

Integrate the four commits currently on `github/master` into the local
`master` branch, preserving the existing local tip, then push the resulting
history to `origin/master`.

## Source commits

- `687ce88400521f698f066d860a6aaff6fb091ca7`
- `81a00670464663af24039bd95e2b2b72eb91c1bd`
- `d54afe3c6ce19d9282c34f56099142ad7721f0ea`
- `087a176eeb502b6451fe81cf4efd3a9458f2c4b5`

## Procedure

1. Confirm the checkout, clean status, remotes, ancestry, and source commit scope.
2. Cherry-pick the four source commits in chronological order onto local `master`.
3. Run `cargo check` and inspect the resulting history and staged/worktree state.
4. Push the integrated local `master` to `origin/master` and verify remote ancestry.

## Safety

- Do not force-push or rewrite the GitHub source branch.
- Stop and resolve any cherry-pick conflict explicitly before pushing.
- If validation fails, do not push the result.

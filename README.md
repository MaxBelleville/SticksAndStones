# Server setup guideline

Spacetime db doesn't have rust based 'server' right now it's two exes.
So we have to manually call the exes when, and add it to the target debug until they add it. (lib is planned but not implemented according to discord)

1. Copy the spacetime.wasm from spacetime target, to target debug
2. Find the current spacetimedb simlink copy both files to target debug
3. Ensure that the spacetime cli db is up to date

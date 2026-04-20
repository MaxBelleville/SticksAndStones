@echo off

spacetime build
spacetime generate --lang rust --out-dir src/net/stdb_bindings  --module-path spacetimedb
ROBOCOPY "%LOCALAPPDATA%/SpacetimeDB/bin/current/" "%cd%/target/debug/"
copy "%cd%\spacetimedb\target\wasm32-unknown-unknown\release\spacetime.wasm" "%cd%\target\debug\"

#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# test.cmd for Linux: builds the .NET solution and runs its tests, then the native engine's (cargo test --profile
# checked over the workspace at the root, debug assertions on, which plays every scenario of scenarios/ whose cards it
# has).
#
#   test.sh                     the fast tests: everything that does not run on the wall clock
#   test.sh --all               every test, the wall-clock ones included
#   test.sh --noweb             every test but the browser tests
#   test.sh --nobuild           skip the build; test what is already built
#   test.sh --engine            the rules engine's tests alone (cargo test over the workspace, the scenario corpus
#                               included): no .NET build or test
#   test.sh --filter "FullyQualifiedName~Swiss"
#                               only matching .NET tests, passed to dotnet test --filter as is; the native engine's
#                               tests are left out
#   test.sh --detailed          print each test and full failure messages
#
# It takes whatever dotnet is on PATH. Without one installed, run it inside
# `nix shell nixpkgs#dotnet-sdk_10 -c tools/test.sh`. The test projects target net8.0; DOTNET_ROLL_FORWARD=Major
# (the default here) lets them run on a newer runtime when 8 is not installed.
#
# The web client (Uno WebAssembly) needs the wasm-tools workload. Without it the whole solution cannot build, so the
# build falls back to dotnet/Cantrip.NoWeb.slnf and says so; the fast run leaves the browser tests out anyway.
#
# The .NET tests that play a table spawn the cantrip-native-host the cards profile last built, so the build step
# builds it too (`cargo build --profile cards -p cantrip-host`): a stale one plays without a card added since.
#
# Exits non-zero if the build or any test fails, and always ends with one verdict line: "All tests passed.",
# "Tests failed." or "Build failed. No tests ran."
# ---------------------------------------------------------------------------
set -u

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD=1
ENGINE=
WEB=1
SLOW=
FILTER=
FILTERED=
DETAILED=()

# No MSBuild worker nodes left behind as idle processes after the run.
export MSBUILDDISABLENODEREUSE=1
export DOTNET_CLI_TELEMETRY_OPTOUT=1
export DOTNET_ROLL_FORWARD="${DOTNET_ROLL_FORWARD:-Major}"

usage() {
  sed -n '6,16p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
  exit 1
}

while [ $# -gt 0 ]; do
  case "$1" in
    --nobuild)  BUILD= ;;
    --engine)   ENGINE=1 ;;
    --noweb)    WEB= ; SLOW=1 ;;
    --all)      SLOW=1 ;;
    --filter)   [ $# -ge 2 ] || usage; FILTER="$2"; FILTERED=1; shift ;;
    --detailed) DETAILED=(--logger "console;verbosity=detailed") ;;
    -h|--help)  usage ;;
    *)          echo "Unknown option: $1"; usage ;;
  esac
  shift
done

# A filter given is the only filter: dotnet test takes one. Without one, the default leaves out the wall-clock tests,
# and --noweb only the browser's.
if [ -z "$FILTER" ]; then
  if [ -z "$SLOW" ]; then
    FILTER="Category!=WallClock&FullyQualifiedName!~Cantrip.Client.Web.Tests"
  elif [ -z "$WEB" ]; then
    FILTER="FullyQualifiedName!~Cantrip.Client.Web.Tests"
  fi
fi

SOLUTION="$ROOT/dotnet/Cantrip.sln"
if [ -z "$WEB" ]; then
  SOLUTION="$ROOT/dotnet/Cantrip.NoWeb.slnf"
elif ! dotnet workload list 2>/dev/null | grep -q '^ *wasm-tools'; then
  echo "No wasm-tools workload: building dotnet/Cantrip.NoWeb.slnf, without the web client."
  SOLUTION="$ROOT/dotnet/Cantrip.NoWeb.slnf"
fi

RESULT=0
BUILDFAILED=

if [ -n "$BUILD" ] && [ -z "$ENGINE" ]; then
  dotnet build "$SOLUTION" -v q --nologo || BUILDFAILED=1
  if [ -z "$BUILDFAILED" ]; then
    (cd "$ROOT" && cargo build --profile cards -p cantrip-host --quiet) || BUILDFAILED=1
  fi
fi

if [ -n "$BUILDFAILED" ]; then
  RESULT=1
else
  if [ -z "$ENGINE" ]; then
    FILTERARGS=()
    [ -n "$FILTER" ] && FILTERARGS=(--filter "$FILTER")
    dotnet test "$SOLUTION" --no-build --nologo "${FILTERARGS[@]}" "${DETAILED[@]}" || RESULT=1
  fi

  # The rules engine: its unit tests and the scenario corpus, with the engine's own consistency checks on. Run from
  # the workspace's directory so its rust-toolchain.toml chooses the toolchain. A --filter names dotnet tests, so it
  # leaves these out.
  if [ "$RESULT" = 0 ] && [ -z "$FILTERED" ]; then
    (cd "$ROOT" && CANTRIP_VERIFY=1 cargo test --profile checked --workspace --quiet) || RESULT=1
  fi
fi

echo
if [ "$RESULT" = 0 ]; then
  echo "All tests passed."
elif [ -n "$BUILDFAILED" ]; then
  echo "Build failed. No tests ran."
else
  echo "Tests failed."
fi
exit "$RESULT"

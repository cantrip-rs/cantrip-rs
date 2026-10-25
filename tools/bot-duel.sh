#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# bot-duel.cmd for Linux: plays one way of playing a seat against another, many times, and says which won more.
#
#   bot-duel.sh                                  the tactical bot against the random one, 400 games of RW aggro
#   bot-duel.sh --a neural --b tactical --deck rw-aggro --deck ub-reanimator --games 400
#                                                the network (gen0, Full information, network only) against the tactical
#                                                bot: the AI player's gate (docs/ai-player-plan.md, docs/ai-host.md)
#   bot-duel.sh --a neural@256 --b neural --deck rw-aggro --deck ub-reanimator --mirrors --games 1600 --parallel 16 --sprt
#                                                search (64 simulations an answer) against the same network alone,
#                                                stopping when the SPRT (H0 50%, H1 55%) has its verdict: Phase 3's gate
#   bot-duel.sh --a neural:hidden --model ../ai-data/models/gen1 --ai-backend tract
#   bot-duel.sh watch                            one game, told turn by turn
#
# It builds the native host (cantrip-native-host) and, for a neural seat, the AI host (cantrip-ai-host, with ONNX
# Runtime) first, then BotLab, and passes every argument through; run with --help for the rest. It takes the dotnet
# on PATH, or runs inside `nix shell nixpkgs#dotnet-sdk_10` when there is none; DOTNET_ROLL_FORWARD=Major lets the
# net8.0 tool run on a newer runtime.
# ---------------------------------------------------------------------------
set -eu

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMMAND=duel
if [ "${1:-}" = "watch" ]; then
    COMMAND=
    shift
fi

cargo build --manifest-path "$ROOT/Cargo.toml" --profile cards -p cantrip-host --bin cantrip-native-host
case " $* " in
    *" neural"*) cargo build --manifest-path "$ROOT/Cargo.toml" --profile cards -p cantrip-ai-host --features ort ;;
esac
export CANTRIP_NATIVE_HOST="${CANTRIP_NATIVE_HOST:-$ROOT/target/cards/cantrip-native-host}"
export DOTNET_ROLL_FORWARD="${DOTNET_ROLL_FORWARD:-Major}"

PROJECT="$ROOT/dotnet/tools/Cantrip.BotLab/Cantrip.BotLab.csproj"
if command -v dotnet > /dev/null; then
    DOTNET=(dotnet)
else
    DOTNET=(nix shell nixpkgs#dotnet-sdk_10 -c dotnet)
fi
"${DOTNET[@]}" build "$PROJECT" -c Release -v q --nologo
exec "${DOTNET[@]}" "$ROOT/dotnet/tools/Cantrip.BotLab/bin/Release/net8.0/cantrip-botlab.dll" $COMMAND "$@"

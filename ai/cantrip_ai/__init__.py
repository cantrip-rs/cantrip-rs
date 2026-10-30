"""The AI player's network, trainer and export (docs/ai-player-plan.md, "Network" and "Training").

- `model`: the network over `cantrip_obs::encode` positions (value, policy over the step's options, the auxiliary
  life-swing head, and the belief head, which is trained only on Hidden and Decklist samples).
- `dataset`: shards (ai/shards.py) decoded to the encoder's tensors and batched by length, each shard's card rows
  mapped onto one training index.
- `embed`: the card-text table a network's card rows start from, fitted when it is first trained and kept by its
  checkpoint; nothing about cards is checked in.
- `train`: imitation of the recorded answers with the value and auxiliary targets; writes `checkpoint.pt`,
  `metrics.json`, `train.log` and `reference.bin` (held-out positions) to `--out`.
- `export`: an ONNX graph per token bucket (32, 64, 128; opset 17, TorchScript exporter, checked against an operator
  allow-list), `cards.json` (the card names the model's table rows are), `manifest.json` (schema, encoder and
  card index hashes, metrics), and `reference/` (positions with PyTorch's outputs, checked through onnxruntime here
  and through tract and `ort` in Rust: `cargo test -p cantrip-ai --test eval -- --ignored`), and `model.pt`, the
  weights the inference server runs.
- `serve`: the inference server, the network on the GPU for every self-play lane and AI host at once
  (`cantrip_ai::remote::RemoteEvaluator`; docs/ai-training.md, "The GPU inference server").

Run from `ai/`, in a Python environment with PyTorch (docs/ai-training.md sets one up for CUDA, ROCm or the CPU):

    python -m cantrip_ai.train --data ../../ai-data/gen0/matchup ... --out DIR [--epochs N] [--hours H]
    python -m cantrip_ai.export --checkpoint DIR/checkpoint.pt --out MODEL_DIR

A model directory is what `cantrip_ai::eval::TractEvaluator::load` (or `OrtEvaluator`, `--features ort`) loads.
"""

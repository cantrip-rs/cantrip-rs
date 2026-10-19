"""Positions a second for the spike model in PyTorch, per bucket: CPU on one thread and on every thread, and CUDA or
ROCm (torch.cuda covers both) when a GPU is there.

    python bench_torch.py [--seconds S]
"""

import argparse
import os
import time

import torch

import model as spike
from export import BATCHES, TOKENS


def rate(net, inputs, seconds, sync):
    with torch.inference_mode():
        for _ in range(3):
            net(*inputs)
        sync()
        runs, start = 0, time.perf_counter()
        while time.perf_counter() - start < seconds:
            net(*inputs)
            runs += 1
        sync()
        return runs * inputs[0].shape[0] / (time.perf_counter() - start)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--seconds", type=float, default=1.0)
    seconds = parser.parse_args().seconds
    net = spike.build()
    targets = [("cpu, 1 thread", "cpu", 1), (f"cpu, {os.cpu_count()} threads", "cpu", os.cpu_count())]
    if torch.cuda.is_available():
        targets.append((f"gpu ({torch.cuda.get_device_name(0)})", "cuda", None))
    else:
        print("No CUDA or ROCm device visible to torch.")
    print(f"torch {torch.__version__}, {spike.parameters(net)} parameters; positions/s")
    print("target".ljust(28) + "".join(f"T{t}xB{b}".rjust(11) for t in TOKENS for b in BATCHES))
    for label, device, threads in targets:
        if threads:
            torch.set_num_threads(threads)
        sync = torch.cuda.synchronize if device == "cuda" else (lambda: None)
        on = net.to(device)
        row = []
        for tokens in TOKENS:
            for batch in BATCHES:
                inputs = [x.to(device) for x in spike.positions(batch, tokens, seed=tokens)]
                row.append(rate(on, inputs, seconds, sync))
        print(label.ljust(28) + "".join(f"{r:11.0f}" for r in row), flush=True)


if __name__ == "__main__":
    main()

# The worker manager

`cantrip-workers` (`tools\cantrip-workers.cmd`, `tools/cantrip-workers`; the code is `tools/workers/cantrip_workers`,
Python's standard library and nothing else) is the one way anything here gets and uses a compute machine: this
machine, another on the network, or a rented vast.ai instance. Code and agents alike ask it for a **lease**, and reach
the machine only through it:

```
cantrip-workers lease "os=linux,gpu" --holder bench-agent --purpose "search benchmark" --json
cantrip-workers exec LEASE -- nvidia-smi
cantrip-workers push LEASE build/bundle.tar /work/bundle.tar
cantrip-workers pull LEASE /work/results.json results.json
cantrip-workers release LEASE
```

Three rules hold it together:

1. **Leases are the only way in.** A machine is busy exactly while a live lease holds it. Nothing is guessed from what
   runs on a machine or which files changed.
2. **Access goes through the tool.** A holder is told a machine's name, kind, size and root, never its address, user
   or key. Every command and every copy is a call of the tool's under a lease; the tool starts the system's own `ssh`
   or `scp` with what only the pool knows, and writes the call down.
3. **The pool is local.** Machines, addresses, keys, the vast.ai account, budgets and the lease state live in a
   per-user directory outside the repository. Nothing about any machine or person belongs in the repository.

It lives in `tools/` and not in `ai/` because it is not the AI's: the training driver is one holder among others
(`ai/cantrip_ai/distributed.py`, "The training driver" below), and it must run where there is no PyTorch.

## The local directory

`CANTRIP_WORKERS_HOME`, else `%LOCALAPPDATA%\cantrip-workers` on Windows and `~/.config/cantrip-workers` elsewhere
(`cantrip-workers home` prints it; `--home DIR` overrides it for one command).

| File | What |
|---|---|
| `pool.json` | the machines, the SSH key, the vast.ai limits: written by hand, by `init` (an example that reaches and rents nothing) and by `import` |
| `state.json` | the live leases and the rented instances; written whole and renamed, under `state.lock` |
| `ledger.jsonl` | every rental and destruction with its price and duration; appended, never rewritten |
| `log.jsonl` | every lease, end, exec, push, pull, forward and sweep, with its lease, machine, holder, exit code and time (`cantrip-workers log`) |
| `known_hosts` | the machines' host keys, apart from the user's own file |
| `vast-api-key` | the vast.ai API key (or `VAST_API_KEY`) |
| `daemon.lock`, `daemon.log` | the daemon's ("Rented machines") |

## `pool.json`

```json
{
  "key": "keys/workers_ed25519",
  "lease_seconds": 900,
  "machines": [
    {"name": "here", "local": true, "os": "windows", "root": "D:/compute", "cores": 32, "ram_gb": 64, "gpu": "RTX 4080"},
    {"name": "worker1", "host": "192.0.2.10", "user": "worker", "os": "windows", "root": "D:/compute",
     "cores": 64, "ram_gb": 128, "gpu": "RTX 4080", "worker": {"lanes": 200}}
  ],
  "vast": {
    "auto_rent": false, "max_dph": 1.00, "daily_budget": 10, "total_budget": 100, "max_instances": 1,
    "idle_minutes": 15, "watchdog_minutes": 20,
    "filters": {"gpu_name": "RTX 5090", "num_gpus": 1, "gpu_max_power": {"gte": 550},
                "cpu_cores_effective": {"gte": 60}, "cpu_ram": {"gte": 120000}, "reliability": {"gt": 0.98}},
    "cpu_names": ["EPYC", "Threadripper"], "order": "price_per_core"
  }
}
```

- `key`: the private key the machines are reached with (a machine's own `key` wins); a path, relative to the directory.
  Renting also needs its public half beside it (`KEY.pub`), which is put on a rented box.
- `lease_seconds`: how long a lease lasts unless renewed (`--ttl` on a lease says otherwise).
- A machine: `name`; `os` (`windows` or `linux`); `root`, where its users' files go on it; `host`, `user` and
  optionally `port`, or `"local": true` for this machine itself (no SSH: its commands run here); `cores`, `ram_gb`,
  `gpu`, what it has, for requests that ask; `worker`, anything its users want handed back with a lease, passed on
  unread (the training driver's `lanes`, `server_args`, ...); `enabled: false` to leave it out; anything else is notes.
- `vast`: "Rented machines" below. Without it, or with any of its four limits at 0, nothing is ever rented.

## Requests

A request is groups joined by `+`, each a comma list:

| Request | Means |
|---|---|
| `worker1,worker2` | those machines, each whole |
| `all` | every enabled remote machine that is free now, each whole (rented boxes included; this machine never) |
| `os=linux,gpu,cores=32` | any one machine that fits: `os=`, `gpu` or `gpu=MODEL`, `ram=GB`, `cores=N`, `count=N` of them, `remote` or `local` for where; this machine first, then the pool's, then boxes already rented |
| `...,rent` | when no machine fits, rent one that does, within the limits and only if the pool has `auto_rent`. A `gpu=MODEL` that may rent is the model's whole name as vast.ai has it (`gpu=RTX_5090`) |
| `...,optional` | take nothing for the group rather than fail: a machine that is busy is passed over, and a rental that is refused or fails is done without |

`cores=N` asks for a share: N of a machine's cores, which other shares can sit beside until its cores are all taken
(and `gpu` in a share takes the machine's GPU, one holder at a time). Without `cores` a lease is the whole machine and
needs it empty. A request is granted whole or not at all: exit 71 and nothing held when a group that is not
`optional` cannot be served. A holder that asks again is given the machines it already holds first. The leases a
request already has are kept alive while a box is rented for it, which can take many minutes.
`cantrip-workers machines REQUEST --json` says what a request could be served by without leasing anything.

## Leases

`lease REQUEST --holder WHO --purpose WHAT [--ttl S] [--no-sweep] [--json]` prints, with `--json`,

```json
{"workers": [{"lease": "L3f9c...", "name": "worker1", "os": "windows", "root": "D:/compute", "cores": 64,
              "ram_gb": 128, "gpu": "RTX 4080", "lanes": 200, "expires": 1795283263.9}]}
```

one record a machine: the lease's id, the machine's name, kind, root and size, and its `worker` settings. The id is
all a holder needs, and all that is asked of it.

- A lease ends when it expires or is released. `renew LEASE...` puts the end off by the lease's length; a holder renews
  on a timer for as long as it works (a third of the length is a good interval). A lease that ended is never brought
  back: ask again.
- **The same holder asking again gets its own lease back**, renewed - so a process that was restarted finds its
  machines still its own. Two jobs that must not share a lease use two holder names.
- When a lease ends, the calls running under it are stopped (each watches its own lease, and ends its whole process
  tree), its forwards are taken down, and on a Linux machine every process started under it is killed, detached ones
  too: `exec` runs the command, its shell included, with `CANTRIP_LEASE` in its environment
  (`exec env CANTRIP_LEASE=... "$SHELL" -c '...'`), and the sweep kills what carries it. `--no-sweep` takes a lease
  without that, for a holder that manages its own processes (the training driver). On a Windows machine a detached
  process is not found again: do not leave any.
- A lease nobody renews is ended when it expires, not when someone next runs a command: the daemon ("Rented
  machines") runs for as long as any lease is live, started by the command that took it, and sweeps what expires.
- `status` shows every machine, who holds it and why, the rented boxes and what was spent; `--json` for code.

## Access

| Command | Does |
|---|---|
| `exec LEASE [--timeout S] -- COMMAND...` | runs the command line on the machine (the words joined by spaces, as ssh joins them), with the caller's own stdin, stdout and stderr; its exit code |
| `push LEASE SRC DST [-r]` | copies a file (a directory with `-r`) of this machine's to a path on the machine |
| `pull LEASE SRC DST [-r]` | copies a file or directory from the machine here |
| `forward LEASE LOCAL:REMOTE` | forwards this machine's loopback port to the machine's loopback port until the lease ends (`LOCAL` 0: a free port); prints the local port. `unforward LEASE LOCAL` ends it |

The tool is not in the data path: `ssh` and `scp` are started with the caller's handles and do the copying
themselves, so a bundle of hundreds of megabytes costs what scp costs. Host keys are kept in the directory's own
`known_hosts`: a host seen for the first time is trusted and remembered, one whose key changed is refused; a rented
box is known by a name of its own, since its address may have been another box's.

**Exit codes.** `exec`, `push` and `pull` return the command's own (255: the machine could not be reached). The
tool's: 70 no live lease, or the lease ended under the call; 71 no machine for the request; 72 a rental the limits
refuse; 73 the pool's configuration; 74 vast.ai; 124 a call's `--timeout`; 2 usage; 1 anything else, said in a line.
A command run by `exec` can exit 70 too, so the tool's own 70 always comes with `[lease-ended]` on stderr, which no
command writes: code that has to tell the two apart looks for it.

### What this is and is not

On one account this is a convention, not a security boundary: whoever can run the tool can read its directory. What
makes it hold for agents is their permissions. For Claude Code, in `.claude/settings.local.json` (per machine,
gitignored; replace `<user>`):

```json
{
  "permissions": {
    "allow": [
      "Bash(tools/cantrip-workers *)",
      "PowerShell(tools\\cantrip-workers.cmd *)",
      "PowerShell(.\\tools\\cantrip-workers.cmd *)"
    ],
    "deny": [
      "Bash(ssh *)", "Bash(scp *)", "Bash(sftp *)",
      "PowerShell(ssh *)", "PowerShell(scp *)", "PowerShell(sftp *)",
      "Read(//c/Users/<user>/AppData/Local/cantrip-workers/**)",
      "Read(~/AppData/Local/cantrip-workers/**)",
      "Read(~/.config/cantrip-workers/**)",
      "Read(~/.ssh/**)",
      "Edit(//c/Users/<user>/AppData/Local/cantrip-workers/**)",
      "Edit(~/AppData/Local/cantrip-workers/**)",
      "Edit(~/.config/cantrip-workers/**)"
    ]
  }
}
```

Deny wins over allow, in any settings file. The rules match the command an agent writes, so they stop `ssh host` and
`cd x && ssh host`, not a program that opens the key file itself: they keep an agent on the path, they do not confine
one that means to leave it. If `CANTRIP_WORKERS_HOME` points elsewhere, deny that path too. Harder walls, none of them
built: SSH certificates the tool signs for the length of a lease (a leaked address and key then stop working by
themselves), or the tool and its directory under a separate account that agents reach only through a service.

## Rented machines

vast.ai is built in (`cantrip_workers/vast.py`, its REST API over HTTPS). A rented box is a machine like the others
for as long as it exists: Linux, `root` its user, the pool's `vast.root` its root, `vast.worker` its `worker`.

**The limits** (`pool.json`'s `vast`), all of which must be set before anything is rented:

| Setting | Means |
|---|---|
| `max_dph` | the most a box may cost an hour, storage included |
| `daily_budget`, `total_budget` | dollars a UTC day over every box, and in all over the whole ledger. A box is rented only if both cover `min_rental_hours` (1) of it and of the boxes already rented or being rented (a rental under way reserves `max_dph` until it has its offer's price); once either is spent every box is destroyed, leased or not: a budget is a hard limit |
| `max_instances` | boxes at once |
| `auto_rent` | whether a lease request that says `rent` may rent (default no: then only `vast rent`, by hand, does) |
| `idle_minutes` | a box with no live lease this long is destroyed (15) |
| `watchdog_minutes` | a box that has not heard from the tool this long destroys itself (20) |
| `start_minutes` | a box not running, reachable and armed this long after it was created is destroyed (30) |
| `filters` | what an offer has to have, in vast.ai's own fields and units (`cpu_ram` in MB, `disk_space` in GB): a bare value is equality, `{"gte": 550}` a comparison (`eq neq gt gte lt lte in notin`). Verified, rentable, on-demand offers only |
| `cpu_names` | regular expressions; the offer's physical CPU has to match one |
| `order` | `price` (the cheapest), or `price_per_core` (dollars an hour per effective core) |
| `image`, `disk_gb`, `root`, `worker` | the box's image, disk, root, and what its leases hand back |

The settings are checked when the pool is read: each of its kind and in its range, and `watchdog_minutes` long
enough for three heartbeats and two minutes. **The budgets count `dph_total`**, the hourly price vast.ai quotes for
the machine and its disk. Bandwidth is billed on top by the gigabyte, and what vast.ai actually charges is its own
account's to say: leave headroom in the budgets, and in the account's credit.

**Renting** (`vast rent [REQUEST]`, or a lease request with `rent`): the limits are checked; the box's place is
taken in the state, at the most it may cost, before anything is asked of vast.ai, so two rentals at once can neither
both pass `max_instances` nor both fit a budget only one fits; an `intent` line with the label the instance will
carry (`cantrip-<name>-<random>`, one rental's alone) is written to the ledger; the offers are searched and checked
again here (an offer vast.ai says is gone, its `no_such_ask`, is passed over for the next); the instance is created
with that label and the watchdog as its start script; the rental is written to
the ledger at once; the key is put on it; and it is taken only when it runs, answers over SSH, and its watchdog says
it can destroy the box. A box that fails any of that is destroyed by its id, and the request fails. A create whose
answer never came may have made an instance all the same: whatever on the account carries the rental's label is
destroyed, then and at the daemon's every look until nothing can still come of it.
`vast offers [REQUEST]` shows what would be rented, best first, and rents nothing.

**The reaper** destroys a box no live lease has held for `idle_minutes` (counted from when its last lease ended or
expired, whenever that is noticed), every box once a budget is spent, and one whose rental was abandoned. A box is
marked `destroying` in the state, under its lock, before vast.ai is asked: from that moment it is no machine (nothing
can lease it, its leases end) and it is told nothing more, so if its destruction keeps failing here its own watchdog
ends it. Gone is what vast.ai says is gone: a destroy that answered anything but success or its own not-found (a bare
404 is also what a wrong key gets) leaves the box in the books, and it is tried again at every look.
`cantrip-workers reap` does it now.

**The daemon** (`cantrip-workers daemon`) is what looks: every `heartbeat_seconds` (60) it ends and sweeps leases
that expired, tells each box the tool is there, and reaps; every tenth round, and at once while the ledger holds
anything unaccounted for, it checks the books against the account (and when it cannot - the key refused, vast.ai
down - `status` says so). A rental starts it the moment it takes its place, before vast.ai is asked for anything, and
a lease starts it too: detached, in the directory of the command that started it; every command starts it again if there is something to watch and it is not
running; it ends only when nothing is rented, no lease is live and nothing is unaccounted for. A round that fails -
the pool's file half written (the last good one stands), vast.ai down, a bug - is logged in `daemon.log` and the next
one runs. `CANTRIP_WORKERS_NO_DAEMON=1` starts none (tests; a machine that runs `cantrip-workers daemon` its own way).

**The watchdog** is on the box, because a box bills while this machine is asleep or the tool is down. It is the
instance's start script: a loop that destroys the box's own instance once the heartbeat file the daemon touches has
not been touched for `watchdog_minutes`, using the instance's own key and id, which vast.ai puts in the container's
environment (`CONTAINER_API_KEY`, `CONTAINER_ID`). At its start it asks vast.ai for its own instance with that key
and writes whether that worked; the tool reads it before it takes the box, so a box that could not stop its own bill
is never used. Its calls to vast.ai have time limits, and each attempt to destroy itself is written to its log with
vast.ai's answer; the daemon reads the log's last line with every heartbeat, and `status` shows it - a box that woke
this machine's tool again after trying and failing to destroy itself says so. Only destruction stops billing (a
stopped instance still pays for its storage), so nothing here ever stops a box.

**The account.** `vast show` (and the daemon) checks the books against the account's instances:

- a box missing from the account's list is asked for by its id; only if vast.ai says there is none is it destroyed
  once more to be sure and written off (its watchdog destroyed it, or vast.ai did), its cost counted to when it was
  last heard from plus the watchdog's wait;
- an instance carrying a label this pool's ledger issued that no box holds is this pool's, forgotten - a create
  whose answer was lost, a process killed between the create and the books: it is taken into the ledger and
  destroyed; a rental the ledger has open with no box is closed the same way;
- any other instance labelled `cantrip-...` is another pool's (another machine may share the account): it is named
  in the daemon's log at every check and by `status`, and never destroyed.

`vast destroy NAME` destroys a box now.

**The cost ledger** (`vast ledger`, `ledger.jsonl`): an `intent` line before vast.ai is asked for an instance (the
label it will carry), a `rent` line when it is created (its id, label, offer, price an hour and what the machine is),
a `destroy` line when it ends (hours, cost, and why: `idle`, `budget`, `manual`, `failed-start`, `gone`, `orphan`),
and an `abandon` line for an intent nothing came of. What was spent, today and in all, is computed from it; `status`
shows it.

## The training driver

`python -m cantrip_ai.generation --workers REQUEST` (docs/ai-training.md, "Distributed self-play") is a holder like
any other. For each distributed stage - self-play, a duel, the pair miner's judgements - it leases the request's
machines as `generation:<root tag>` (`distributed.Leases`), renews each lease every four minutes while the stage
runs, and releases them when it ends, so a machine is the run's only while it works on it and a rented box idle
through training is reaped. It asks for every group as `optional`, and for a group that says what a machine has to
have as `remote` (`distributed.driver_request`): a stage is played on what can be leased - a machine someone else
holds, or a box that could not be rented, is logged and done without, never a stage that fails - and this machine
plays as itself, not as a worker of its own run. A lease that ended under it (this machine slept) is asked for again;
a renewal that failed some other way is tried again at the next; only a worker whose own lease is refused stops
being the stage's. Every command it runs on a worker and every file it copies is a `cantrip-workers exec`, `push` or
`pull` under the worker's lease (`SshHost.answer`); a lease that ended is a worker that cannot be reached, which its
coordinators already handle. Its leases are taken with `--no-sweep`: a worker's runs are detached, end by the driver's
own heartbeat (`--dist-lease`), and are re-attached to by a driver that comes back. The driver's ledgers, grants and
attempts are its own and unchanged.

`CANTRIP_WORKERS_CLI` (a JSON list, or words) says where the command is, for a driver not run from this repository.

## Moving an old workers file over

Before the pool, the driver read a workers file (`{"key": ..., "workers": [...]}`). Once, on the machine that has it:

```
cantrip-workers import PATH\workers.json --skip-disabled --dry-run
cantrip-workers import PATH\workers.json --skip-disabled
cantrip-workers status
```

Each worker becomes a machine: `name`, `host`, `port`, `user`, `os` (Windows when the file did not say), `root`,
`key`, `enabled`, `gpu`, `ram_gb` and `notes` as they were, `cpu_threads` as `cores`, and everything else - the
driver's settings for it - under `worker`. Then `--workers all` (or the machines' names) in place of
`--workers PATH\workers.json`.

## Tests

`python -m unittest discover -s tools/workers/tests`: the leases, the calls, renting, the reaper, the watchdog's
script and the ledger, with a clock the tests move, an ssh that records what it was asked and a vast.ai that keeps its
instances in a dict. Nothing in them reaches a network or waits on time, and nothing rents.

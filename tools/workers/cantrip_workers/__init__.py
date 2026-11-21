"""The worker manager (docs/workers.md): the one way to get and use a compute machine - this one, another on the
network, a rented one - through a lease. `cli` is the command (`tools/cantrip-workers`), `manager.Manager` the leases
and the rented machines' life, `access.Access` the calls a lease allows, `pool` the configuration and requests, `vast`
vast.ai, `store` the local directory everything is kept in. Standard library only."""

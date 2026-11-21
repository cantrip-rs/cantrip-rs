"""What keeps a rented box from billing unseen, each case a way it once could (the review of the worker manager):
a create whose answer is lost, a process killed mid-rental, a daemon that stopped, a "gone" that was not, two rentals
racing a budget, a lease granted on a box being destroyed. The same fakes as test_workers: no network, no waiting.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from cantrip_workers import cli, vast  # noqa: E402
from cantrip_workers.manager import CREATING_GRACE  # noqa: E402
from cantrip_workers.pool import parse_request  # noqa: E402
from cantrip_workers.store import ConfigError, LimitError, Refused, Unmet, append_line  # noqa: E402
from test_workers import POOL, Case, offer  # noqa: E402


class Money(Case):
    def setUp(self):
        super().setUp()
        self.vast.offers = [offer(2, 0.80), offer(1, 0.95, cores=128)]
        self.logged: list[str] = []
        self.m.log = self.logged.append

    def rented(self):
        return self.store.read_state()["rented"]

    # B1: an instance is never forgotten.

    def test_a_create_whose_answer_is_lost_leaves_no_instance_behind(self):
        self.vast.lose_create = True
        with self.assertRaises(vast.VastError):
            self.m.rent()
        # It was made all the same: found by the label the intent recorded, taken into the ledger, destroyed.
        self.assertEqual((self.vast.instances_, self.vast.destroyed, self.rented()), ({}, [7001], {}))
        intent, rent, destroy = self.ledger("intent", "rent", "destroy")
        self.assertEqual((rent["instance"], rent["label"], destroy["instance"], destroy["by"]), (7001, intent["label"], 7001, "orphan"))
        self.assertTrue(self.daemons)

    def test_an_instance_that_shows_up_after_its_create_failed_is_found_by_the_daemon(self):
        self.vast.lose_create = True
        self.vast.hidden = {7001}  # not on the account's list yet when the rental gives up
        with self.assertRaises(vast.VastError):
            self.m.rent()
        self.assertEqual((set(self.vast.instances_), self.rented()), ({7001}, {}))
        self.assertTrue(self.m.unsettled())
        self.assertTrue(self.m.tick())  # still nothing listed: the intent stays open, and the daemon stays up
        self.vast.hidden = set()
        self.clock.now += 60
        self.m.tick()
        self.assertEqual((self.vast.instances_, self.vast.destroyed), ({}, [7001]))
        self.assertEqual([e["by"] for e in self.ledger("destroy")], ["orphan"])
        # Nothing came of the intent after that: once nothing still can, it is closed and the daemon can end.
        self.clock.now += CREATING_GRACE + 1
        self.assertFalse(self.m.tick())
        self.assertFalse(self.m.unsettled())

    def test_a_rental_killed_between_its_create_and_the_books_is_found_and_destroyed(self):
        # What a killed process leaves: its place in the state, its intent, and an instance nothing here knows by id.
        label = "cantrip-vast1-0badf00d"
        state = self.store.read_state()
        state["rented"]["vast1"] = dict(status="creating", at=self.clock.now, label=label, dph=1.10)
        self.store.write_state(state)
        append_line(self.store.ledger, dict(event="intent", at=self.clock.now, name="vast1", label=label))
        self.vast.instances_[7050] = dict(id=7050, label=label, dph_total=0.8, created=self.clock.now, start_date=self.clock.now)
        # While the rental could still be under way it is left alone, by the reaper and by the account's check.
        self.clock.now += 60
        self.assertEqual(self.m.reap(), [])
        self.assertEqual(self.m.reconcile()["orphans"], [])
        self.assertEqual(set(self.vast.instances_), {7050})
        self.clock.now += CREATING_GRACE
        self.assertEqual(self.m.reap(), ["vast1"])
        self.assertEqual((self.vast.instances_, self.rented()), ({}, {}))
        rent, destroy = self.ledger()
        self.assertEqual((rent["instance"], rent["dph"], destroy["instance"]), (7050, 0.8, 7050))
        self.assertAlmostEqual(destroy["cost"], 0.8 * (60 + CREATING_GRACE) / 3600, places=3)

    def test_an_instance_under_a_label_this_pool_issued_with_no_box_is_destroyed_and_a_strangers_never(self):
        self.m.rent()
        label = self.rented()["vast1"]["label"]
        self.vast.instances_[7090] = dict(id=7090, label=label, dph_total=0.5, created=0, start_date=self.clock.now)
        self.vast.instances_[7091] = dict(id=7091, label="cantrip-vast1-ffffffff", dph_total=0.5, created=0)  # another pool's
        r = self.m.reconcile()
        self.assertEqual((r["orphans"], [s["instance"] for s in r["strays"]]), ([7090], [7091]))
        self.assertEqual(set(self.vast.instances_), {7001, 7091})
        self.assertIn("vast1", self.rented())

    # B2: the daemon does not die.

    def test_the_daemon_outlives_a_pool_file_that_is_wrong_for_a_while(self):
        self.m.rent()
        (lease,) = self.m.lease("vast1", "driver", "a stage", ttl=3600)
        good = self.store.pool.read_text()
        self.store.pool.write_text("{ half a file")
        rounds = []

        def sleep(seconds):
            rounds.append(seconds)
            self.clock.now += seconds
            if len(rounds) == 3:
                self.store.pool.write_text(good)
            if len(rounds) == 5:
                self.m.release([lease["lease"]])

        self.m.sleep = sleep
        self.remote.calls.clear()
        self.assertEqual(cli.run_daemon(self.m, once=False), 0)
        # It ran on through the bad file at its last good interval, kept telling the box it was there, and ended only
        # when the box, idle at last, was destroyed.
        self.assertGreater(len(rounds), 15)
        self.assertEqual(set(rounds), {60.0})
        self.assertGreaterEqual(sum(1 for _, line in self.remote.calls if "touch" in line), 5)
        self.assertIn("not usable", (self.store.home / "daemon.log").read_text())
        self.assertEqual((self.rented(), self.vast.instances_), ({}, {}))

    def test_the_daemon_goes_on_when_it_starts_with_no_usable_pool_and_when_a_round_throws(self):
        self.m.rent()
        m = self.manager()  # a fresh process: no last good pool
        self.store.pool.write_text("[]")
        boom = [RuntimeError("a bug"), None]
        real, slept = m.heartbeat, []

        def heartbeat():
            e = boom.pop(0) if boom else None
            if e:
                raise e
            real()

        def sleep(seconds):
            slept.append(seconds)
            self.clock.now += seconds
            if len(slept) == 3:
                self.store.pool.write_text(json.dumps(POOL))

        m.heartbeat, m.sleep = heartbeat, sleep
        self.assertEqual(cli.run_daemon(m, once=False), 0)
        text = (self.store.home / "daemon.log").read_text()
        self.assertIn("RuntimeError: a bug", text)
        self.assertEqual(self.rented(), {})  # and it came to reap the idle box once the pool was back

    # B3: gone means vast.ai said so.

    def test_a_404_is_not_gone(self):
        answers = [(404, "Not Found"), (404, {"detail": "Invalid user key"}), (404, {"success": False, "error": "not_found"}),
                   (404, "Not Found"), (200, {"instances": None})]
        api = vast.Api("KEY", request=lambda *a, **k: answers.pop(0), sleep=lambda s: None)
        self.assertFalse(api.destroy(5))
        self.assertFalse(api.destroy(5))
        self.assertTrue(api.destroy(5))
        with self.assertRaises(vast.VastError):
            api.show(5)
        self.assertIsNone(api.show(5))

    def test_a_box_missing_from_one_listing_is_not_written_off(self):
        self.m.rent()
        self.vast.hidden = {7001}
        r = self.m.reconcile()
        self.assertEqual((r["gone"], r["orphans"]), ([], []))
        self.assertEqual((self.rented()["vast1"]["status"], set(self.vast.instances_), self.vast.destroyed), ("ready", {7001}, []))
        self.assertEqual(len(self.ledger()), 1)

    # S1: two rentals at once cannot both fit a budget only one fits.

    def test_a_rental_under_way_reserves_its_price_against_the_budgets(self):
        self.write_pool(POOL, max_instances=2, max_dph=1.0, daily_budget=1.5)
        second = []
        search = self.vast.search

        def racing(query):  # a second rental starts while the first is still choosing its offer
            self.vast.search = search
            try:
                second.append(self.m.rent())
            except Refused as e:
                second.append(e)
            return search(query)

        self.vast.search = racing
        self.assertEqual(self.m.rent()["name"], "vast1")
        self.assertIsInstance(second[0], LimitError)
        self.assertRegex(str(second[0]), "daily budget")
        self.assertEqual(len(self.vast.created), 1)
        # Its place holds the price it was rented at from then on, and a budget that covers two takes two.
        self.assertEqual(self.rented()["vast1"]["dph"], 0.80)
        self.write_pool(POOL, max_instances=2, max_dph=1.0, daily_budget=2.0)
        self.assertEqual(self.m.rent()["name"], "vast2")

    # S2, S3: a box being destroyed is no machine, and is told nothing.

    def test_no_lease_is_granted_on_a_box_between_the_decision_to_destroy_it_and_its_destruction(self):
        self.m.rent()
        self.clock.now += 16 * 60
        seen = []

        def racing(instance):  # another holder asks for it while vast.ai is being told to destroy it
            try:
                seen.append(self.m.lease("vast1", "late", "too late"))
            except Refused as e:
                seen.append(e)

        self.vast.on_destroy = racing
        self.assertEqual(self.m.reap(), ["vast1"])
        self.assertIsInstance(seen[0], ConfigError)  # there is no such machine any more
        self.assertEqual(self.store.read_state()["leases"], {})

    def test_a_box_whose_destruction_keeps_failing_is_told_nothing_so_its_watchdog_ends_it(self):
        self.m.rent()
        (w,) = self.m.lease("vast1", "holder", "work", ttl=7200)
        self.write_pool(POOL, daily_budget=0.5)
        self.clock.now += 3600
        self.vast.refuse_destroy = True
        self.remote.calls.clear()
        for _ in range(5):
            self.clock.now += 60
            self.assertTrue(self.m.tick())
        # One heartbeat at most (the round that found the budget spent), then silence; its lease ended at once.
        self.assertLessEqual(sum(1 for _, line in self.remote.calls if "touch" in line), 1)
        self.assertEqual((self.rented()["vast1"]["status"], self.rented()["vast1"]["why"]), ("destroying", "budget"))
        self.assertFalse(self.m.live(w["lease"]))
        with self.assertRaises(Refused):
            self.m.lease("vast1", "other", "x")
        self.vast.refuse_destroy = False
        self.m.tick()
        self.assertEqual((self.rented(), self.ledger()[-1]["by"]), ({}, "budget"))

    # S4: the ledger and the account agree whatever the state missed.

    def test_a_rental_the_state_could_not_take_is_destroyed_by_its_id_and_closed_in_the_ledger(self):
        def failing(name, label, box):
            raise OSError("disk full")

        self.m.place_box = failing
        with self.assertRaises(OSError):
            self.m.rent()
        self.assertEqual((self.vast.instances_, self.vast.destroyed), ({}, [7001]))
        self.assertEqual([(e["event"], e["instance"]) for e in self.ledger()], [("rent", 7001), ("destroy", 7001)])
        self.clock.now += 5 * 3600
        self.assertAlmostEqual(vast.spend(self.ledger(), self.clock.now)["total"], 0.0, places=3)
        self.assertEqual(self.m.reap(), [])  # nothing left over in the state either

    # The second review (N1-N8).

    def test_an_offer_vast_says_is_gone_is_passed_over_and_a_bare_404_is_not_that(self):
        gone_404 = (404, {"success": False, "error": "invalid_args", "ask_id": 2,
                          "msg": "error 404/3603: no_such_ask Instance type by id 2 is not available."})
        gone_410 = (410, {"success": False, "error": "no_such_ask", "ask_id": 2,
                          "msg": "error 410/3907: no_such_ask Instance type 2 is no longer available."})
        answers = [gone_404, gone_410, (404, "Not Found"), (400, {"success": False, "error": "invalid_args", "msg": "error 400/3467: Invalid args"})]
        api = vast.Api("KEY", request=lambda *a, **k: answers.pop(0), sleep=lambda s: None)
        self.assertIsNone(api.create(2, {}))
        self.assertIsNone(api.create(2, {}))
        for _ in range(2):
            with self.assertRaises(vast.VastError):
                api.create(2, {})
        # A rental meeting a stale offer takes the next, with nothing left open behind it.
        real = vast.Api("KEY", request=lambda *a, **k: gone_404, sleep=lambda s: None)
        create = self.vast.create
        self.vast.create = lambda offer, body: real.create(offer, body) if offer == 2 else create(offer, body)
        self.assertEqual(self.m.rent()["dph"], 0.95)
        self.assertFalse(self.m.unsettled())
        self.assertEqual([e["event"] for e in self.ledger("intent", "rent", "abandon")], ["intent", "rent"])

    def test_something_watches_from_the_moment_a_rental_has_its_place(self):
        seen = []
        search = self.vast.search

        def searching(query):  # the first thing asked of vast.ai: a daemon was asked for before it
            seen.append(list(self.daemons))
            return search(query)

        self.vast.search = searching
        self.m.rent()
        self.assertEqual(seen, [[1]])

    def test_a_rental_that_writes_its_line_while_the_account_is_checked_is_not_taken_for_an_orphan(self):
        from cantrip_workers import manager

        label = "cantrip-vast1-0badf00d"
        real = manager.read_lines
        raced = []

        def racing(path):
            # Another process's rental, between this check's listing of the account and its reading of the ledger:
            # its place, its intent, its instance and its `rent` line - the box not in the state yet.
            if not raced and path == self.store.ledger:
                raced.append(1)
                state = self.store.read_state()
                state["rented"]["vast1"] = dict(status="creating", at=self.clock.now, label=label, dph=0.8)
                self.store.write_state(state)
                self.vast.instances_[7060] = dict(id=7060, label=label, dph_total=0.8, created=self.clock.now)
                append_line(self.store.ledger, dict(event="intent", at=self.clock.now, name="vast1", label=label))
                append_line(self.store.ledger, dict(event="rent", at=self.clock.now, instance=7060, name="vast1", label=label, dph=0.8))
            return real(path)

        manager.read_lines = racing
        try:
            r = self.m.reconcile()
        finally:
            manager.read_lines = real
        self.assertEqual((raced, r["orphans"], self.vast.destroyed, set(self.vast.instances_)), ([1], [], [], {7060}))

    def test_a_place_taken_down_stops_its_rental_and_is_never_brought_back(self):
        # Condemned before the create (its destruction not yet through): nothing is asked of vast.ai.
        search = self.vast.search

        def condemning(query):
            state = self.store.read_state()
            state["rented"]["vast1"].update(status="destroying", why="manual")
            self.store.write_state(state)
            return search(query)

        self.vast.search = condemning
        with self.assertRaisesRegex(Refused, "taken down while it was being rented"):
            self.m.rent()
        self.assertEqual(self.vast.created, [])
        self.vast.search = search
        self.m.reap()
        self.assertEqual(self.rented(), {})
        # Taken down during the create, and its name taken by another rental: the instance that then arrives is
        # destroyed by its id, and the other rental's place is not written over.
        create = self.vast.create

        def taken(offer, body):
            self.m.destroy("vast1")
            state = self.store.read_state()
            state["rented"]["vast1"] = dict(status="creating", at=self.clock.now, label="cantrip-vast1-another0", dph=1.1)
            self.store.write_state(state)
            return create(offer, body)

        self.vast.create = taken
        with self.assertRaisesRegex(Refused, "taken down while it was being rented"):
            self.m.rent()
        self.assertEqual((self.vast.instances_, len(self.vast.destroyed)), ({}, 1))
        self.assertEqual(self.rented()["vast1"]["label"], "cantrip-vast1-another0")
        self.assertEqual([e["event"] for e in self.ledger()], ["rent", "destroy"])

    def test_a_daemon_about_to_end_looks_once_more_for_work_that_found_its_lock_held(self):
        (w,) = self.m.lease("lin1", "agent", "a job", ttl=3600)
        self.m.release([w["lease"]])
        rounds = []
        real = self.m.tick

        def tick(reconcile=False):
            rounds.append(1)
            more = real(reconcile)
            if len(rounds) == 1:
                # A lease taken just as this round found nothing to watch: its command found the daemon's lock
                # held, and started none.
                self.assertFalse(more)
                self.late = self.m.lease("lin1", "late", "a job", ttl=90)[0]
            return more

        self.m.tick = tick
        self.assertEqual(cli.run_daemon(self.m, once=False), 0)
        # It went on, and ended only when that lease had expired and been swept.
        self.assertGreater(len(rounds), 2)
        self.assertFalse(self.m.live(self.late["lease"]))
        self.assertEqual([e["why"] for e in self.events("end")], ["released", "expired"])

    def test_books_the_account_cannot_settle_are_said_in_the_status(self):
        append_line(self.store.ledger, dict(event="rent", at=self.clock.now, instance=7070, name="vast7", dph=1.0))

        def down():
            raise vast.VastError("vast.ai: 401 unauthorized")

        self.vast.instances = down
        self.assertTrue(self.m.tick())
        s = self.m.status()
        self.assertEqual(s["unsettled"], dict(error="vast.ai: 401 unauthorized"))
        self.assertTrue(any("cannot: vast.ai: 401" in line for line in cli.status_lines(s | dict(forwards=[]))))
        del self.vast.instances  # the account answers again: settled, and no longer said
        self.assertFalse(self.m.tick())
        self.assertNotIn("unsettled", self.m.status())

    def test_a_lease_that_expired_is_not_revived_by_keeping_a_requests_leases_alive(self):
        (w,) = self.m.lease("win1", "driver", "a stage", ttl=60)
        self.clock.now += 61  # expired; nothing has pruned it yet
        state = self.m.touch([w["lease"]])
        self.assertLess(state["leases"][w["lease"]]["expires"], self.clock.now)
        self.assertFalse(self.m.live(w["lease"]))
        self.m.lease("win1", "other", "its own")

    def test_a_ledger_rental_with_no_box_is_closed_destroyed_if_it_is_there(self):
        append_line(self.store.ledger, dict(event="rent", at=self.clock.now, instance=7070, name="vast7", dph=1.0))
        append_line(self.store.ledger, dict(event="rent", at=self.clock.now, instance=7071, name="vast8", dph=1.0))
        self.vast.instances_[7070] = dict(id=7070, label="cantrip-vast7", dph_total=1.0, created=0)
        self.clock.now += 1800
        self.assertTrue(self.m.unsettled())
        self.assertFalse(self.m.tick())  # reconciled at once: one destroyed, the other confirmed gone
        self.assertEqual((self.vast.instances_, self.vast.destroyed), ({}, [7070]))
        self.assertEqual(sorted((e["instance"], e["by"]) for e in self.ledger("destroy")), [(7070, "orphan"), (7071, "gone")])
        self.clock.now += 10 * 3600
        self.assertAlmostEqual(vast.spend(self.ledger(), self.clock.now)["total"], 1.0, places=2)

    # S5: the watchdog's own calls end, and what it did is seen.

    def test_the_watchdogs_calls_time_out_and_its_last_word_is_in_the_status(self):
        text = vast.watchdog_script(self.m.pool.vast)
        self.assertIn("--connect-timeout 20 --max-time 60", text)
        self.assertIn("timeout=60", text)
        self.assertLess(len(text), 4048)
        self.m.rent()
        self.remote.watchdog_log = "2026-01-01T00:00:00+00:00 no heartbeat for 1300 s: destroying instance 7001: 403\n"
        self.m.heartbeat()
        self.assertIn("destroying instance 7001: 403", self.m.status()["rented"][0]["watchdog"])

    # T3, T5 and the request's rentals.

    def test_the_leases_a_request_already_has_are_kept_alive_while_its_box_is_rented(self):
        self.write_pool(POOL, auto_rent=True)
        self.vast.runs_after = 20 * 60  # twenty minutes to start; the leases last one
        got = self.m.lease("win1+gpu=RTX_5090,os=linux,rent", "driver", "a stage", ttl=60)
        self.assertEqual([w["name"] for w in got], ["win1", "vast1"])
        self.assertGreater(self.clock.now, 20 * 60)
        for w in got:
            self.assertTrue(self.m.live(w["lease"]), w["name"])
            self.assertAlmostEqual(w["expires"], self.clock.now + 60, delta=1)
        self.assertEqual([e["why"] for e in self.events("end")], [])

    def test_a_holder_that_asks_again_has_its_own_machines_before_any_other(self):
        (mine,) = self.m.lease("win1,cores=16", "driver", "a stage")
        (again,) = self.m.lease("cores=16,remote", "driver", "a stage")  # lin1 comes first by name
        self.assertEqual((again["name"], again["lease"]), ("win1", mine["lease"]))
        self.write_pool(POOL, auto_rent=True)
        (box,) = self.m.lease("gpu=RTX_5090,rent", "driver", "a stage")
        (back,) = self.m.lease("gpu=RTX_5090,rent", "driver", "a stage")
        self.assertEqual((back["lease"], len(self.vast.created)), (box["lease"], 1))

    def test_an_optional_group_whose_rental_fails_goes_without(self):
        self.write_pool(POOL, auto_rent=True)
        self.vast.offers = []
        got = self.m.lease("win1,optional+gpu=RTX_5090,rent,optional", "driver", "a stage")
        self.assertEqual([w["name"] for w in got], ["win1"])
        self.assertTrue(any("no box was rented" in line for line in self.logged))
        self.write_pool(POOL, auto_rent=True, daily_budget=0.5)
        self.vast.offers = [offer(2, 0.80)]
        self.assertEqual([w["name"] for w in self.m.lease("win1,optional+gpu,os=linux,rent,optional", "driver", "a stage")], ["win1"])
        with self.assertRaises(LimitError):
            self.m.lease("gpu,os=linux,rent", "other", "a stage")

    # A2: the daemon is what ends a lease nobody renews.

    def test_the_daemon_stays_while_a_lease_is_live_and_sweeps_it_when_it_expires(self):
        (w,) = self.m.lease("lin1", "agent", "a job", ttl=300)
        self.assertTrue(self.m.tick())
        self.assertEqual(self.remote.calls, [])
        self.clock.now += 301
        self.assertFalse(self.m.tick())  # expired: ended, its processes killed, and nothing left to watch
        self.assertEqual([name for name, _ in self.remote.calls], ["lin1"])
        self.assertIn(f"CANTRIP_LEASE={w['lease']}$", self.remote.calls[0][1])
        self.assertEqual([e["why"] for e in self.events("end")], ["expired"])

    # Nits.

    def test_a_rental_taken_down_while_it_started_is_an_error_said_plainly(self):
        self.vast.runs_after = 100

        def remote(m, line, timeout=60.0):  # the box answers; meanwhile someone destroyed it by hand
            if "vast1" in self.rented() and self.rented()["vast1"]["status"] == "starting":
                self.m.destroy("vast1")
            return 0, "armed 1\n"

        self.m.remote = remote
        with self.assertRaisesRegex(Refused, "taken down while it started"):
            self.m.rent()
        self.assertEqual((self.rented(), self.vast.instances_), ({}, {}))

    def test_rental_settings_of_the_wrong_kind_or_out_of_range_are_refused(self):
        for setting, value, message in (("max_dph", "1.10", "max_dph is a number"), ("daily_budget", -1, "of at least 0"),
                                        ("max_instances", 1.5, "whole number"), ("idle_minutes", 0, "idle_minutes"),
                                        ("watchdog_minutes", 3, "too short"), ("heartbeat_seconds", 1, "heartbeat_seconds"),
                                        ("auto_rent", "yes", "true or false"), ("order", "cheapest", "price or"),
                                        ("cpu_names", ["("], "regular expression"), ("filters", [], "an object"),
                                        ("api_url", "http://example.invalid", "https"), ("min_rental_hours", 0, "above 0")):
            self.write_pool(POOL, **{setting: value})
            with self.assertRaisesRegex(ConfigError, message, msg=setting):
                self.manager().pool  # noqa: B018
        self.write_pool(POOL, watchdog_minutes=5, heartbeat_seconds=60)
        self.manager().pool  # noqa: B018

    def test_a_requests_gpu_model_is_asked_of_vast_itself(self):
        self.write_pool({**POOL, "vast": {k: v for k, v in POOL["vast"].items() if k != "filters"}})
        (g,) = parse_request("gpu=RTX_4090")
        q = vast.offer_query(self.m.pool.vast, g)
        self.assertEqual(q["gpu_name"], {"eq": "RTX 4090"})
        # The pool's own filter stands when it names one; a request for another model then finds nothing.
        self.write_pool(POOL)
        self.assertEqual(vast.offer_query(self.m.pool.vast, g)["gpu_name"], {"eq": "RTX 5090"})
        with self.assertRaises(Unmet):
            self.m.rent(g)

    def test_a_lease_that_is_not_live_is_said_with_a_mark_no_command_writes(self):
        import contextlib
        import io

        from cantrip_workers.store import LEASE_ENDED

        err = io.StringIO()
        with contextlib.redirect_stderr(err), contextlib.redirect_stdout(io.StringIO()):
            code = cli.main(["--home", str(self.store.home), "exec", "Lnone", "--", "true"], manager=self.m)
        self.assertEqual(code, 70)
        self.assertIn(LEASE_ENDED, err.getvalue())

    def test_an_error_nobody_foresaw_is_a_line_and_a_code_not_a_traceback(self):
        import contextlib
        import io

        def broken(*a, **k):
            raise KeyError("vast1")

        self.m.status = broken
        err = io.StringIO()
        with contextlib.redirect_stderr(err), contextlib.redirect_stdout(io.StringIO()):
            code = cli.main(["--home", str(self.store.home), "status"], manager=self.m)
        self.assertEqual((code, err.getvalue().strip()), (1, "cantrip-workers: KeyError: 'vast1'"))

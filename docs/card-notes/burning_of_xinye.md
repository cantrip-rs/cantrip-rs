Owner's decision (jschibel, 2026-11-28): when Burning of Xinye is implemented, the same change adds a by-name special
case for it to `crates/cantrip-cards/src/cards/equinox.rs`, beside Cleansing's, that skips Equinox's speculative
resolution ("Counter target spell if it would destroy a land you control", `Query::speculate_resolution`). The closed-form
rule is worked out then, cited to the CR and the rulings, and proven against the probe at small sizes the way
`crates/cantrip-script/tests/equinox_cleansing_closed_form.rs` proves Cleansing's.

Why: the probe tries every order of every set of four lands each player picks, and the target opponent's picks come after
the caster's, so the work is about three times the product of the two picks' orders. A test-only copy of the text
measured (checked profile, 2026-11-28): caster 5 lands and opponent 6 is 43,200 outcomes in 1.5 s, judged; 6 lands each
is past the probe's bound (`Unjudged` after 87,120 outcomes in 4.5 s), and Equinox would wrongly let it resolve
(docs/known-issues.md, KI-179).

A starting point for the rule - UNVERIFIED, to be checked against the CR, the rulings and the probe before it is used:
Equinox counters Burning of Xinye when the ability's controller is one of the two players who destroy lands (its caster,
or its target opponent, who must still be a legal target) and that player cannot fill their four picks (or all of their
lands, with fewer than four) with lands that would not really be destroyed - lands that can be chosen (not indestructible,
per the 2013-07-01 ruling) but whose destruction a regeneration shield, a shield counter, an umbra or another replacement
would stop. How many of a player's lands would really be destroyed can be read off one speculative outcome, as Equinox
does for Cleansing; check that the picks' order cannot change it.

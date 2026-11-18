use cantrip_api::*;
use cantrip_mechanics::{effects, solved, targets, triggers, your_creatures_have_total_power_at_least};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        // The targets and their shares, each at least one, are announced as it is put on the stack (CR 601.2d by way of
        // 603.3d); a target illegal as it resolves takes its share with it (CR 608.2b).
        triggered(
            triggers::THIS_ENTERS,
            effects::distribute_plus_one_counters,
            "When this Case enters, distribute two +1/+1 counters among one or two target creatures you control.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL.between(1, 2)])
        .divide(Divide { total: 2, reads: None }),
        // "To solve — Creatures you control have total power 8 or greater" (CR 719.3a): an intervening "if" (CR 603.4).
        solved::to_solve(
            your_creatures_have_total_power_at_least::<8>,
            "To solve \u{2014} Creatures you control have total power 8 or greater. (If unsolved, solve at the beginning of your end step.)",
        ),
        solved::triggered(
            triggers::YOU_ATTACK,
            effects::plus_one_counters_on_first_target_that_gains_until_end_of_turn::<1, { Keywords::of(&[Keyword::Trample]).0 }>,
            "Solved \u{2014} Whenever you attack, put a +1/+1 counter on target attacking creature. It gains trample until end of turn.",
        )
        .targets(&[targets::ATTACKING_CREATURE]),
    ],
    ..oracle::CASE_OF_THE_TRAMPLED_GARDEN
};

use cantrip_api::*;
use cantrip_mechanics::{cumulative_upkeep, sacrifice_chosen, triggers};
use cantrip_oracle as oracle;

/// "When this land enters, sacrifice a land." (CR 701.21a).
fn sacrifice_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    sacrifice_chosen(g, item.controller, item.object, 1, |c| c.is(CardType::Land)).map(drop)
}

/// "Creatures you control can't attack." (CR 508.1c).
fn cumulative_upkeep_pay_2_life_rule(
    q: &dyn Query,
    asker: &Asker,
    attacker: ObjectId,
    _: Target,
) -> bool {
    let asker = &Asker::new(asker.source, q.controller(asker.source));
    !((q.chars(attacker).is(CardType::Creature)
        && asker.controller == q.controller(attacker)))
}

/// "Prevent all damage that would be dealt to you." (CR 615.1).
fn cumulative_upkeep_pay_2_life_applies(
    _: &dyn Query,
    asker: &Asker,
    e: &DamageEvent,
) -> bool {
    {
        let target = &e.target;
        if matches!(* target, Target::Player(p) if asker.controller == p) {
            return true;
        }
    }
    false
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        cumulative_upkeep!(
            cantrip_mechanics::cumulative_upkeep::Payment::Life(2), "Cumulative upkeep—Pay 2 life. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)"
        ),
        triggered(
            triggers::THIS_ENTERS,
            sacrifice_land,
            "When this land enters, sacrifice a land.",
        ),
    ],
    preventions: &[
        PreventionDef {
            hint: "prevents all damage that would be dealt to you",
            applies: cumulative_upkeep_pay_2_life_applies,
            amount: PreventionAmount::All,
        },
    ],
    attack_rules: &[cumulative_upkeep_pay_2_life_rule],
    ..oracle::GLACIAL_CHASM
};

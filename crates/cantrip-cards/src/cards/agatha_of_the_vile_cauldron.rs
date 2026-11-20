use cantrip_api::*;
use cantrip_mechanics::{effects, statics};
use cantrip_oracle as oracle;

/// "Activated abilities of creatures you control cost {X} less to activate, where X is Agatha's power. This effect can't reduce the
/// mana in that cost to less than one mana" (CR 601.2f, 602.2b, 118.7): X is her power as the cost is worked out, none when it is
/// negative; a creature only counts on the battlefield, so a cycling cost or another activated ability from a hand is not reduced. A
/// triggered ability has no cost to activate.
fn creature_abilities_cost_her_power_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    match statics::permanent_whose_ability_you_activate(q, source, player, costed) {
        Some(creature) if q.chars(creature).is(CardType::Creature) => {
            CostDelta::less_to_one(q.chars(source).power.unwrap_or(0).max(0) as u32)
        }
        _ => CostDelta::NONE,
    }
}

/// "Other creatures you control get +1/+1 and gain trample and haste until end of turn" (CR 611.2c): the creatures you control as the
/// ability resolves but Agatha, none that enter later.
fn other_creatures_get_plus_one_and_gain_trample_and_haste(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::pump_other_creatures_you_control_and_gain(g, item, 1, 1, &[Keyword::Trample, Keyword::Haste]);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{4}{R}{G}"),
        other_creatures_get_plus_one_and_gain_trample_and_haste,
        "{4}{R}{G}: Other creatures you control get +1/+1 and gain trample and haste until end of turn.",
    )],
    cost_changes: &[creature_abilities_cost_her_power_less],
    ..oracle::AGATHA_OF_THE_VILE_CAULDRON
};

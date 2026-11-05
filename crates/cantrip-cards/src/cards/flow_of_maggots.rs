use cantrip_api::*;
use cantrip_mechanics::cumulative_upkeep;
use cantrip_oracle as oracle;

/// "This creature can't be blocked by non-Wall creatures." (CR 604.1).
fn creature_cant_be_blocked_by_rule(
    q: &dyn Query,
    source: ObjectId,
    blocker: ObjectId,
    attacker: ObjectId,
) -> bool {
    !(attacker == source
        && q.chars(blocker).is(CardType::Creature)
        && !q.chars(blocker).has_subtype(q.words(source).subtype(Subtype::Wall)))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        cumulative_upkeep!(
            cumulative_upkeep::Payment::Mana(mana("{1}")),
            "Cumulative upkeep"
        ),
    ],
    block_rules: &[creature_cant_be_blocked_by_rule],
    ..oracle::FLOW_OF_MAGGOTS
};

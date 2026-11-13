use cantrip_api::*;
use cantrip_mechanics::{cumulative_upkeep, history};
use cantrip_oracle as oracle;

/// "Creatures that attacked during their controller's last turn can't attack." (CR 604.1).
fn creatures_attacked_during_their_controllers_rule(
    q: &dyn Query,
    _: &Asker,
    attacker: ObjectId,
    _: Target,
) -> bool {
    !(q.chars(attacker).is(CardType::Creature)
        && history::attacked_during_last_turn_of(q, q.controller(attacker), attacker))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        cumulative_upkeep!(
            cumulative_upkeep::Payment::Mana(mana("{1}")), "Cumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)"
        ),
    ],
    attack_rules: &[creatures_attacked_during_their_controllers_rule],
    ..oracle::HALLS_OF_MIST
};

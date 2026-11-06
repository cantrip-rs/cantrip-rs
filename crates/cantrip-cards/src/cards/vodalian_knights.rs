use cantrip_api::*;
use cantrip_mechanics::{effects, lands};
use cantrip_oracle as oracle;

/// The defending player, including a battle's protector, must control an Island (CR 508.5).
fn attack_rule(q: &dyn Query, asker: &Asker, attacker: ObjectId, defender: Target) -> bool {
    attacker != asker.source || defending_player_of(q.state(), defender)
        .is_none_or(|player| lands::controls_land_type(q, player, q.words(asker.source).subtype(Subtype::Island)))
}

/// The controller's lack of Islands creates a state trigger (CR 603.8).
fn no_islands(q: &dyn Query, asker: &Asker) -> bool {
    !lands::controls_land_type(q, asker.controller, q.words(asker.source).subtype(Subtype::Island))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        state_triggered(no_islands, effects::sacrifice_self, "When you control no Islands, sacrifice this creature."),
        ability(cost("{U}"), effects::this_gains_flying, "{U}: This creature gains flying until end of turn."),
    ],
    attack_rules: &[attack_rule],
    ..oracle::VODALIAN_KNIGHTS
};

use cantrip_api::*;
use cantrip_mechanics::{effects, lands};
use cantrip_oracle as oracle;

/// The defending player must control a Forest (CR 508.5).
fn attack_rule(q: &dyn Query, asker: &Asker, attacker: ObjectId, defender: Target) -> bool {
    attacker != asker.source || defending_player_of(q.state(), defender)
        .is_none_or(|player| lands::controls_land_type(q, player, q.words(asker.source).subtype(Subtype::Forest)))
}

/// The controller's lack of Forests creates a state trigger (CR 603.8).
fn no_forests(q: &dyn Query, asker: &Asker) -> bool {
    !lands::controls_land_type(q, asker.controller, q.words(asker.source).subtype(Subtype::Forest))
}

pub static CARD: CardDef = CardDef {
    abilities: &[state_triggered(no_forests, effects::sacrifice_self,
        "When you control no Forests, sacrifice this creature.")],
    attack_rules: &[attack_rule],
    ..oracle::GORILLA_PACK
};

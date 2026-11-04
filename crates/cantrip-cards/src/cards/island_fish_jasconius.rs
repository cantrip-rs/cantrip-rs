use cantrip_api::*;
use cantrip_mechanics::{effects, lands, triggers};
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

fn pay_to_untap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_pay_to_untap_self(g, item, cost("{U}{U}{U}"))
}

pub static CARD: CardDef = CardDef {
    keywords: Keywords::of(&[Keyword::DoesNotUntap]),
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            pay_to_untap,
            "At the beginning of your upkeep, you may pay {U}{U}{U}. If you do, untap this creature.",
        ),
        state_triggered(
            no_islands,
            effects::sacrifice_self,
            "When you control no Islands, sacrifice this creature.",
        ),
    ],
    attack_rules: &[attack_rule],
    ..oracle::ISLAND_FISH_JASCONIUS
};

use cantrip_api::*;
use cantrip_mechanics::prevention;
use cantrip_oracle as oracle;

/// "The next time this creature would deal damage to you this turn, prevent that damage." (CR 615.1, 615.8).
fn prevent_next_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    prevention::this_turn(g, item, Target::Player(item.controller), 0, Shield::NextTime);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{3}"), prevent_next_damage,
        "{3}: The next time this creature would deal damage to you this turn, prevent that damage. Any player may activate this ability.")
        .any_player_may_activate()],
    granted_preventions: &[GrantedPreventionDef {
        hint: "prevents the next damage dealt by that permanent",
        text: "The next time this creature would deal damage to you this turn, prevent that damage.",
        ..prevention::DAMAGE_DEALT_BY
    }],
    ..oracle::MERCENARIES
};

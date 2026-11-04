use cantrip_api::*;
use cantrip_mechanics::{combat::during_combat, effects, triggers};
use cantrip_oracle as oracle;

/// "During combat, players can't cast spells" (CR 101.2, 601.3): every player, from every zone, a cast an effect offers
/// during resolution included.
fn no_casting_during_combat(q: &dyn Query, _: &Asker, _: PlayerId, _: ZoneKind) -> bool {
    !during_combat(q)
}

/// "... or activate abilities that aren't mana abilities" (CR 101.2, 602.5): any permanent's activated ability but a mana
/// ability (CR 605.1a), which stays allowed.
fn no_activating_during_combat(q: &dyn Query, _: &Asker, _: ObjectId, def: &ActivatedDef) -> bool {
    !during_combat(q) || def.is_mana_ability()
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when(EventMask::ATTACKS, triggers::creature_you_control_attacks_a_player_alone),
        effects::subject_gains_double_strike_until_end_of_turn,
        "Whenever a creature you control attacks a player alone, it gains double strike until end of turn.",
    )],
    cast_rules: &[no_casting_during_combat],
    activation_rules: &[no_activating_during_combat],
    ..oracle::YURIKO_BLADE_OF_THE_MIGHTY
};

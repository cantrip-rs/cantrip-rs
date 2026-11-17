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
fn no_activating_during_combat(q: &dyn Query, _: &Asker, _: PlayerId, _: ObjectId, def: &ActivatedDef) -> bool {
    !during_combat(q) || def.is_mana_ability()
}

/// "It gains double strike until end of turn" (CR 702.4a, 611.2a): the attacking creature the trigger is about.
fn gain_double_strike(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(creature) = item.subject {
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::DoubleStrike);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when(EventMask::ATTACKS, triggers::creature_you_control_attacks_a_player_alone),
        gain_double_strike,
        "Whenever a creature you control attacks a player alone, it gains double strike until end of turn.",
    )],
    cast_rules: &[no_casting_during_combat],
    activation_rules: &[no_activating_during_combat],
    ..oracle::YURIKO_BLADE_OF_THE_MIGHTY
};

use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Whenever you activate an ability of a creature" (CR 602.2, 605.3, 603.2): any ability of a creature on the battlefield
/// that its controller activated - a mana ability included (the ruling) - the creature as it was when the costs were paid, or
/// as it last was if paying them took it away (`triggers::activation`).
fn you_activate_an_ability_of_a_creature(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let (creature, player, def) = triggers::activation(q, event)?;
    (player == me.controller && def.from == ZoneKind::Battlefield && q.chars(creature).is(CardType::Creature)).then_some(Fired::PLAIN)?
}

/// "Other target nonland permanents you control" (CR 115.1): on the battlefield under this ability's controller's control,
/// not a land, and not Elrond himself.
fn other_nonland_permanent_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::nonland_permanent_you_control_filter(q, asker, t) && t != Target::Object(asker.source)
}

/// "Exile up to two other target nonland permanents you control. Return those cards to the battlefield under their owner's
/// control at the beginning of the next end step." (CR 701.13, 603.7, 400.7): those still legal are exiled together, and the
/// delayed ability is about the cards exiled (CR 603.7c): a token has ceased to exist and does not return, a card that left
/// exile in between is a new object and does not either.
fn flicker(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let exiled = g.exile_all(&targets::legal_objects(g, item));
    delayed::at_next_end_step_about(g, item, 0, &exiled);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ABILITY_ACTIVATED, you_activate_an_ability_of_a_creature),
            |g, item| effects::draw(g, item, 1),
            "Whenever you activate an ability of a creature, draw a card. This ability triggers only once each turn.",
        )
        .triggers_each_turn(1),
        ability(
            cost("{5}{U}{U}"),
            flicker,
            "{5}{U}{U}: Exile up to two other target nonland permanents you control. Return those cards to the battlefield under their owner's control at the beginning of the next end step.",
        )
        .targets(&[target(other_nonland_permanent_you_control).between(0, 2)]),
    ],
    delayed: &[delayed_trigger(delayed::return_exiled_to_battlefield, "Return those cards to the battlefield under their owner's control.")],
    ..oracle::ELROND_MOON_READER
};

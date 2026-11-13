use cantrip_api::*;
use cantrip_mechanics::{effects, lands, targets, triggers};
use cantrip_oracle as oracle;

const DRAIN: Question = Question::new("LoseLife", "pay {cost} so that the target player loses 2 life and you gain 2 life?", "", "Pay {cost}", "Don't pay");

/// "You may pay {B}. If you do, target player loses 2 life and you gain 2 life." (CR 608.2d, 118.12, 119.3): asked as the ability resolves. An ability whose only target is
/// no longer legal does not resolve at all (CR 608.2b), so it never gets here to ask.
fn pay_to_drain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, cost("{B}"), &DRAIN)? {
        effects::target_player_loses_life_you_gain_life(g, item, 2)?;
    }
    Ok(())
}

/// "Whenever this creature or another Vampire you control dies" (CR 700.4, 603.10a): another permanent's subtype is read as it
/// last existed on the battlefield, and this creature triggers on its own death whatever its creature types were then ("this
/// creature", not "this Vampire": one that lost its creature types and kept its abilities still triggers).
fn this_or_another_vampire_you_control_dies(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    triggers::this_dies(q, me.source, event).or_else(|| triggers::another_of_subtype_you_control_dies::<{ lands::sub(Subtype::Vampire) }>(q, me, event))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, this_or_another_vampire_you_control_dies),
        pay_to_drain,
        "Whenever this creature or another Vampire you control dies, you may pay {B}. If you do, target player loses 2 life and you gain 2 life.",
    )
    .targets(&[targets::PLAYER])],
    ..oracle::KALASTRIA_HIGHBORN
};

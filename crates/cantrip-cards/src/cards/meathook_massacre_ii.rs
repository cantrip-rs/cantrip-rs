use cantrip_api::*;
use cantrip_mechanics::each_player_sacrifices_x;
use cantrip_mechanics::triggers::{self, THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Each player sacrifices X creatures of their choice" (CR 701.21a, 101.4, 107.3a): X is what this permanent was cast with
/// (entering without being cast, 0). The active player chooses first, then each other player in turn order, and every chosen
/// creature is sacrificed together; a player with fewer than X creatures sacrifices them all.
fn each_player_sacrifices_x_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = g.obj(item.object).x;
    each_player_sacrifices_x(g, item.object, x, |q, id| q.chars(id).is(CardType::Creature))?;
    Ok(())
}

/// "Whenever a creature you control dies" (CR 700.4, 603.10a), about that card: the creature that died, as the card now in its
/// owner's graveyard (CR 400.7), is the trigger's subject.
fn your_creature_dies(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::ZoneChange { new, .. } = event else { return None };
    triggers::creature_you_control_dies(q, me, event)?;
    Fired::about(Target::Object(*new))
}

/// "Whenever a creature an opponent controls dies" (CR 700.4, 603.10a), about that card as above; `data[0]` is the player who
/// controlled the creature as it died, the "they" who may pay (CR 608.2h).
fn opponents_creature_dies(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::ZoneChange { old, new, .. } = event else { return None };
    triggers::creature_an_opponent_controls_dies(q, me, event)?;
    Some(Fired { subject: Target::Object(*new), data: [u32::from(q.controller(*old).0), 0] })
}

const PAY_LIFE: Question = Question::new("Pay", "pay 3 life?", "", "Pay 3 life", "Don't pay");

/// "Return that card under your control with a finality counter on it" (CR 400.7, 122.6, 122.1h): it enters with the counter,
/// under this ability's controller. A card that has left the graveyard since, or a token that has ceased to exist (CR 111.7),
/// is a new object the engine leaves where it is.
fn return_that_card_with_finality(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(card) = item.subject else { return Ok(()) };
    g.put_onto_battlefield_with_counters(card, item.controller, false, &[(CounterKind::Finality, 1)])?;
    Ok(())
}

/// "You may pay 3 life. If you do, return that card under your control with a finality counter on it" (CR 118.12, 119.4): the
/// life is paid as the ability resolves, whether or not the card is still there to return (the rulings).
fn you_may_pay_3_life_to_return_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, life(3), &PAY_LIFE)? {
        return_that_card_with_finality(g, item)?;
    }
    Ok(())
}

/// "They may pay 3 life. If they don't, return that card under your control with a finality counter on it": the creature's
/// last controller decides, and a player who can't pay (fewer than 3 life, CR 119.4) doesn't.
fn they_may_pay_3_life_or_you_return_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let they = PlayerId(item.data[0] as u8);
    if !g.may_pay(they, item.object, item.object, life(3), &PAY_LIFE)? {
        return_that_card_with_finality(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, each_player_sacrifices_x_creatures, "When Meathook Massacre II enters, each player sacrifices X creatures of their choice."),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, your_creature_dies),
            you_may_pay_3_life_to_return_it,
            "Whenever a creature you control dies, you may pay 3 life. If you do, return that card under your control with a finality counter on it.",
        ),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, opponents_creature_dies),
            they_may_pay_3_life_or_you_return_it,
            "Whenever a creature an opponent controls dies, they may pay 3 life. If they don't, return that card under your control with a finality counter on it.",
        ),
    ],
    ..oracle::MEATHOOK_MASSACRE_II
};

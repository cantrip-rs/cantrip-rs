use cantrip_api::*;
use cantrip_mechanics::{effects, library, targets};
use cantrip_oracle as oracle;

/// I: "Mill five cards. When you do, ..." The reflexive trigger exists only if five cards were milled (CR 701.17b, 603.12), and
/// chooses its target once they are in the graveyard.
fn mill_and_reflex(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let this = g.card_id(&CARD);
    library::mill_then_reflexive(g, item, 5, AbilityRef::delayed(this, 0));
    Ok(())
}

/// "This Saga deals damage equal to the greatest power among creature cards in your graveyard to target creature" (CR 120.3,
/// 208.1): read as the trigger resolves, so the milled cards count, and only the controller's graveyard does; a power of 0 or
/// less (or no creature card) is no damage (CR 107.1b). A target gone by then is left alone (CR 608.2b).
fn damage_equal_to_greatest_power_in_your_graveyard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(victim) = targets::object(g, item, 0) else { return Ok(()) };
    let graveyard = g.state().zone(Zone::graveyard(item.controller)).to_vec();
    let greatest = graveyard.into_iter().map(|card| g.chars(card)).filter(|c| c.is(CardType::Creature)).filter_map(|c| c.power).max().unwrap_or(0);
    if greatest > 0 {
        g.damage(item.object, Target::Object(victim), i64::from(greatest));
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(
            1,
            mill_and_reflex,
            "I \u{2014} Mill five cards. When you do, this Saga deals damage equal to the greatest power among creature cards in your graveyard to target creature.",
        ),
        chapter(2, effects::target_creature_gets::<2, 2>, "II \u{2014} Target creature you control gets +2/+2 until end of turn.")
            .targets(&[targets::CREATURE_YOU_CONTROL]),
        chapter(3, effects::return_first_target_to_hand, "III \u{2014} Return target creature card from your graveyard to your hand.")
            .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD]),
    ],
    delayed: &[delayed_trigger(
        damage_equal_to_greatest_power_in_your_graveyard,
        "When you do, this Saga deals damage equal to the greatest power among creature cards in your graveyard to target creature.",
    )
    .targets(&[targets::CREATURE])],
    ..oracle::KRAVENS_LAST_HUNT
};

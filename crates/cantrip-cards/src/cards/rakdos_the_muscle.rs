use cantrip_api::*;
use cantrip_mechanics::library::exile_top_may_play;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

// Flying and trample are the core's, read from the card's JSON.

/// "Whenever you sacrifice another creature" (CR 701.21a): a creature of yours other than Rakdos that a sacrifice moved from
/// the battlefield, read as it was when it left (CR 603.10a), so a creature sacrificed as a cost counts. About that creature,
/// its mana value in `data[0]` for the ability to exile that many cards.
fn you_sacrifice_another_creature(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let sacrificed = triggers::sacrificed_by(q, me.controller, event)?;
    if sacrificed == me.source || !q.chars(sacrificed).is(CardType::Creature) {
        return None;
    }
    Some(Fired { subject: Target::Object(sacrificed), data: [q.mana_value(sacrificed), 0] })
}

/// "Exile cards equal to its mana value from the top of target player's library. Until your next end step, you may play those
/// cards, and mana of any type can be spent to cast those spells." (CR 609.4b, 305.1, 500.4): the permission is to play, so a land
/// among them may be the turn's land drop, and it ends as the controller's next end step begins.
fn exile_and_play(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = item.target(0).player() else { return Ok(()) };
    let grant = PlayGrant { lands: true, any_type: true, expiry: Expiry::UntilYourNextEndStep, ..PlayGrant::CAST };
    exile_top_may_play(g, player, item.data[0] as usize, item.controller, grant);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, you_sacrifice_another_creature),
            exile_and_play,
            "Whenever you sacrifice another creature, exile cards equal to its mana value from the top of target player's library. Until your next end step, you may play those cards, and mana of any type can be spent to cast those spells.",
        )
        .on_sacrifice()
        .targets(&[targets::PLAYER]),
        ability(
            sacrifice(1, targets::filters::another_creature),
            effects::this_gains_indestructible_then_tap,
            "Sacrifice another creature: Rakdos gains indestructible until end of turn. Tap it. Activate only once each turn.",
        )
        .once_each_turn(),
    ],
    ..oracle::RAKDOS_THE_MUSCLE
};

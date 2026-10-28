use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Tap an untapped Villain you control" (CR 602.2b): any untapped permanent of its controller's with the creature type, a
/// creature that came under their control this turn included (CR 302.6 limits only its own {T} abilities). The word is read
/// through this enchantment's text (CR 612).
fn untapped_villain_you_control(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.controller(id) == asker.controller && !q.obj(id).tapped && q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Villain))
}

/// "Mill a card. Put a plan counter on this enchantment." (CR 701.13, 122.6) The counter goes only on this enchantment
/// while it is still there (CR 400.7).
fn mill_and_plan(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.mill(item.controller, 1);
    g.add_counters(item.object, CounterKind::Plan, 1);
    Ok(())
}

/// "Return target creature card from your graveyard to the battlefield", under its controller's control (CR 110.2). A
/// target gone by now leaves the ability nothing to do (CR 608.2b).
fn reanimate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    g.put_onto_battlefield(card, item.controller, false)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            tap_untapped(1, untapped_villain_you_control),
            mill_and_plan,
            "Tap an untapped Villain you control: Mill a card. Put a plan counter on this enchantment. Activate only as a sorcery.",
        )
        .sorcery_speed(),
        triggered(
            when_asked(EventMask::COUNTERS, triggers::nth_counter_put_on_this::<{ CounterKind::Plan as u8 }, 4>),
            effects::sacrifice_this_when_you_do,
            "When the fourth plan counter is put on this enchantment, sacrifice it. When you do, return target creature card from your graveyard to the battlefield.",
        ),
    ],
    delayed: &[delayed_trigger(reanimate, "When you do, return target creature card from your graveyard to the battlefield.")
        .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD])],
    ..oracle::VILLAINOUS_SYNDICATION
};

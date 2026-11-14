use cantrip_api::*;
use cantrip_mechanics::{effects, lands, targets, triggers, untapped_subtype_you_control};
use cantrip_oracle as oracle;

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
            tap_untapped(1, untapped_subtype_you_control::<{ lands::sub(Subtype::Villain) }>),
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

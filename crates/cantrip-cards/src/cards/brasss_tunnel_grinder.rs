use cantrip_api::*;
use cantrip_mechanics::{discover, effects, history, spent, triggers};
use cantrip_oracle as oracle;

/// "Put a bore counter on this artifact. Then if there are three or more bore counters on it, remove those counters and
/// transform it": the ability has no instruction after the transform that depends on it, so whether there were three is not read.
fn bore_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::counter_then_transform_at_three::<{ CounterKind::Bore as usize }>(g, item)?;
    Ok(())
}

/// "Discover X, where X is that spell's mana value" (CR 701.57a, 202.3e, 608.2h): the value of the spell the ability is about,
/// the trigger's subject, as it is on the stack with the X chosen for it counted, read as the ability resolves - the spell may have
/// resolved before then, as the ruling says its ability resolves first.
fn discover_its_mana_value(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = item.subject.object().map_or(0, |spell| g.mana_value(spell));
    discover::discover(g, item.controller, x)?;
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::R), "{T}: Add {R}."),
        triggered(
            when_asked(EventMask::SPELL_CAST, triggers::permanent_spell_cast_with_this_permanents_mana),
            discover_its_mana_value,
            "Whenever you cast a permanent spell using mana produced by Tecutlan, the Searing Rift, discover X, where X is that spell's mana value.",
        ),
    ],
    mana_told_apart: &[spent::this_permanents_mana_is_told_apart],
    ..oracle::TECUTLAN_THE_SEARING_RIFT
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::discard_any_number_then_draw_that_many_plus_one, "When Brass's Tunnel-Grinder enters, discard any number of cards, then draw that many cards plus one."),
        triggered(
            triggers::YOUR_END_STEP,
            bore_counter,
            "At the beginning of your end step, if you descended this turn, put a bore counter on Brass's Tunnel-Grinder. Then if there are three or more bore counters on it, remove those counters and transform it.",
        )
        .condition(history::descended_this_turn),
    ],
    back: Some(&BACK),
    ..oracle::BRASSS_TUNNEL_GRINDER
};

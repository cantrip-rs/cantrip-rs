use cantrip_api::*;
use cantrip_mechanics::{effects, lands, mana, triggers};
use cantrip_oracle as oracle;

/// A Mountain tapped for mana produces colorless instead (CR 614.1a, 106.12b).
fn mountain_mana(q: &dyn Query, asker: &Asker, _: ObjectId, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ManaProduction(m)
        if q.chars(m.permanent).has_subtype(q.words(asker.source).subtype(Subtype::Mountain)))
}

fn colorless_instead(q: &dyn Query, asker: &Asker, _: ObjectId, proposal: &mut Proposal) {
    mana::produce_instead::<{ Mana::C as u8 }>(q, asker, proposal);
}

/// Count all permanents at resolution, then create the chosen effects until end of turn (CR 611.2a, 611.2c).
fn upkeep(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let odd = g.state().battlefield().len() % 2 == 1;
    let amount = if odd { 1 } else { -1 };
    let red = g.words(item.object).color(Color::Red);
    // One sentence, one instruction: the pump and the mana effect begin together (CR 608.2c).
    instruction(g, |g| {
        effects::pump_creatures_where(g, item.object, |q, id| q.chars(id).colors.has(red), amount, amount);
        if odd {
            g.delayed_trigger(DelayedWhen::ThisTurn, item.object, item.controller,
            AbilityRef::delayed(item.ability.card, 0), Target::None);
        } else {
            g.add_player_effect(item.controller, item.object,
                EffectKind::ReplacementRule { card: item.ability.card, index: 0 }, Duration::EndOfTurn);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::EACH_UPKEEP,
        upkeep,
        "At the beginning of each upkeep, count the number of permanents. If the number is odd, until end of turn, red creatures get +1/+1 and whenever a player taps a Mountain for mana, that player adds an additional {R}. If the number is even, until end of turn, red creatures get -1/-1 and if a player taps a Mountain for mana, that Mountain produces colorless mana instead of any other type.",
    )],
    delayed: &[triggered(
        when_asked(EventMask::TAPPED, mana::a_player_taps_a_land_type_for_mana::<{ lands::sub(Subtype::Mountain) }>),
        mana::that_player_adds_an_additional::<{ Mana::R as u8 }>,
        "Whenever a player taps a Mountain for mana, that player adds an additional {R}.",
    ).mana_ability(TriggeredMana::One(Mana::R))],
    granted_replacements: &[granted_replacement(
        ReplacementKind::ManaProduction, mountain_mana, colorless_instead,
        "If a player taps a Mountain for mana, that Mountain produces colorless mana instead of any other type.",
    ).text("If a player taps a Mountain for mana, that Mountain produces colorless mana instead of any other type.")],
    ..oracle::CHAOS_MOON
};

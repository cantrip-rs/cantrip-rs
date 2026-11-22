use cantrip_api::*;
use cantrip_mechanics::{mana, targets, triggers};
use cantrip_oracle as oracle;

/// "Whenever Ultima attacks, put a blight counter on target land. For as long as that land has a blight counter on it, it
/// loses all land types and abilities and has '{T}: Add {C}.'" (CR 122.1, 611.2b, 305.7, 613.1d, 613.1f, 613.7): the land
/// type is lost in layer 4 (and with it the mana ability a basic land type gives, CR 305.6) and its other card types and
/// supertypes stay (the ruling); its abilities are lost in layer 6 before the mana ability is added, by timestamp.
/// It lasts as long as the counter does, Ultima gone or not, and never begins if no counter is on the land once the
/// counter is put (a replacement that keeps it off, CR 614.1a). Nothing happens if the land is no longer a legal target
/// (CR 608.2b).
fn blight(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(land) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(land, CounterKind::Blight, 1);
    g.end_sentence();
    let card = g.card_id(&CARD);
    let until = Duration::WhileCounter(CounterKind::Blight);
    instruction(g, |g| {
        g.add_effect(item.object, land, EffectKind::LoseSubtypesOf(CardType::Land), until);
        g.add_effect(item.object, land, EffectKind::LoseAllAbilities, until);
        g.add_effect(item.object, land, EffectKind::AddAbility(AbilityRef::delayed(card, 0)), until);
    });
    Ok(())
}

/// "Whenever you tap a land for {C}" (CR 605.1b, 106.12a): a land this ability's controller taps whose mana ability
/// produced colorless mana, the player in `data[0]` for `that_player_adds_an_additional`.
fn you_tap_a_land_for_colorless(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match *event {
        Event::TappedForMana { permanent, player, produced }
            if player == q.controller(source) && produced.get(Mana::C) > 0 && q.chars(permanent).is(CardType::Land) =>
        {
            Some(Fired { subject: Target::Object(permanent), data: [player.0 as u32, 0] })
        }
        _ => None,
    }
}

// Flying is the core's, read from the card's JSON.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ATTACKS,
            blight,
            "Whenever Ultima attacks, put a blight counter on target land. For as long as that land has a blight counter on it, it loses all land types and abilities and has \"{T}: Add {C}.\"",
        )
        .targets(&[targets::LAND]),
        // A triggered mana ability: it resolves at once, without the stack (CR 605.1b, 605.4a; the ruling).
        triggered(
            when(EventMask::TAPPED, you_tap_a_land_for_colorless),
            mana::that_player_adds_an_additional::<{ Mana::C as u8 }>,
            "Whenever you tap a land for {C}, add an additional {C}.",
        )
        .mana_ability(TriggeredMana::One(Mana::C)),
    ],
    delayed: &[mana_ability(TAP, add(Mana::C), "{T}: Add {C}.")],
    ..oracle::ULTIMA_ORIGIN_OF_OBLIVION
};

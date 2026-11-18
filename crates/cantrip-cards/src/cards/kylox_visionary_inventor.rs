use cantrip_api::*;
use cantrip_mechanics::triggers::{self, ward};
use cantrip_mechanics::{choose, effects, history, select, top_of_library};
use cantrip_oracle as oracle;

/// "Whenever Kylox attacks, sacrifice any number of other creatures, then exile the top X cards of your library, where X is
/// their total power. You may cast any number of instant and/or sorcery spells from among the exiled cards without paying their
/// mana costs." (CR 701.21a, 608.2g, 118.9, 400.7)
///
/// The sacrifice is chosen as the ability resolves (not a cost), any number of the controller's other creatures, none included.
/// X is the sum of the power each sacrificed creature last had on the battlefield (CR 608.2h) - pumps and counters counted, a
/// negative power counted as it is - and cannot be below zero. The top X cards are exiled together (CR 101.4); only those
/// that really reached exile are among "the exiled cards". Each instant or sorcery among them may then be cast for nothing as
/// the ability resolves, in any order, any number of them; one given up stays exiled and a card that is neither is
/// never offered.
fn sacrifice_then_exile_and_cast(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let sacrificed = effects::may_sacrifice(g, you, item.object, select::another_creature_permanents(select::ANY_NUMBER))?;
    let x: i32 = sacrificed.objects().iter().map(|&card| history::power_of_sacrificed_permanent(g, card)).sum();
    let top = top_of_library(g, you, x.max(0) as usize);
    let exiled: Vec<ObjectId> = g.exile_all(&top).into_iter().filter(|&card| g.obj(card).zone.is(ZoneKind::Exile)).collect();
    // "An instant and/or sorcery spell": the face that would be cast is the spell (CR 709.3a, 715.3a), so an Adventure's instant half counts.
    choose::cast_up_to_among(
        g,
        you,
        item.object,
        Purpose::CastFree,
        &exiled,
        usize::MAX,
        CastTerms { instead: Some(ManaCost::ZERO), ..CastTerms::default() },
        |g, w| {
            let face = g.card(w.face);
            face.is(CardType::Instant) || face.is(CardType::Sorcery)
        },
    )
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ward::<2>("Ward {2}"),
        triggered(
            triggers::THIS_ATTACKS,
            sacrifice_then_exile_and_cast,
            "Whenever Kylox attacks, sacrifice any number of other creatures, then exile the top X cards of your library, where X is their total power. You may cast any number of instant and/or sorcery spells from among the exiled cards without paying their mana costs.",
        ),
    ],
    ..oracle::KYLOX_VISIONARY_INVENTOR
};

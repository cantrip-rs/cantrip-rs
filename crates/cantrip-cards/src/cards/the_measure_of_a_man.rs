use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// I: "Exile up to one target nontoken artifact or creature." The card is exiled with this Saga (CR 607.2a), for chapter
/// III to find; with no target, or one gone by resolution, nothing is exiled (CR 608.2b).
fn exile_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(permanent) = targets::object(g, item, 0) {
        g.exile_with(&[permanent], g.linked(item), false);
    }
    Ok(())
}

/// III: "Return the exiled card to the battlefield under its owner's control. You gain 2 life." The card is the one exiled
/// with this Saga, as it is now (CR 607.2a); it returns under its owner's control (CR 110.2a), and the life is gained
/// whether or not there is one.
fn return_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let exiled: Vec<(ObjectId, PlayerId, bool)> = g
        .exiled_with(g.linked(item))
        .into_iter()
        .map(|card| (card, g.state().obj(card).owner, false))
        .collect();
    if !exiled.is_empty() {
        g.put_all_onto_battlefield_as(&exiled)?;
    }
    g.gain_life(item.controller, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(
            1,
            exile_it,
            "I \u{2014} Exile up to one target nontoken artifact or creature.",
        )
        .targets(&[target(targets::nontoken_artifact_or_creature_filter).optional()]),
        chapter(2, tokens::you_investigate, "II \u{2014} Investigate."),
        chapter(
            3,
            return_it,
            "III \u{2014} Return the exiled card to the battlefield under its owner's control. You gain 2 life.",
        ),
    ],
    ..oracle::THE_MEASURE_OF_A_MAN
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];

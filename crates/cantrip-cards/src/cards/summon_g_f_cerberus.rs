use cantrip_api::*;
use cantrip_mechanics::{copy, delayed, effects};
use cantrip_oracle as oracle;

/// "I — Surveil 1." (CR 701.25)
fn surveil_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 1)
}

/// "II — Double — When you next cast an instant or sorcery spell this turn, copy it. You may choose new targets for the copy."
/// (CR 603.7, 707.10): sets up delayed ability 0, `copy::COPY_IT_THE_NEXT_INSTANT_OR_SORCERY_YOU_CAST`.
fn double(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    delayed::next_this_turn(g, item, 0, item.object);
    Ok(())
}

/// "III — Triple — When you next cast an instant or sorcery spell this turn, copy it twice. You may choose new targets for the
/// copies." (CR 603.7, 707.10): sets up delayed ability 1, `copy::COPY_THE_NEXT_INSTANT_OR_SORCERY_YOU_CAST_TWICE`.
fn triple(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    delayed::next_this_turn(g, item, 1, item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, surveil_one, "I \u{2014} Surveil 1."),
        chapter(
            2,
            double,
            "II \u{2014} Double \u{2014} When you next cast an instant or sorcery spell this turn, copy it. You may choose new targets for the copy.",
        ),
        chapter(
            3,
            triple,
            "III \u{2014} Triple \u{2014} When you next cast an instant or sorcery spell this turn, copy it twice. You may choose new targets for the copies.",
        ),
    ],
    delayed: &[copy::COPY_IT_THE_NEXT_INSTANT_OR_SORCERY_YOU_CAST, copy::COPY_THE_NEXT_INSTANT_OR_SORCERY_YOU_CAST_TWICE],
    ..oracle::SUMMON_G_F_CERBERUS
};

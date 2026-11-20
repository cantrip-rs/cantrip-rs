use cantrip_api::*;
use cantrip_mechanics::attach::{attach_to_target, attached_object, equip};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{history, targets};
use cantrip_oracle as oracle;

/// "Celebration — At the beginning of combat on your turn, if two or more nonland permanents entered the battlefield under your
/// control this turn, attach this Equipment to up to one target creature you control." (CR 207.2c, 603.4, 701.3a): an intervening "if",
/// asked as combat begins and again as the ability resolves; with no target chosen, or one no longer legal, nothing is attached.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            BEGINNING_OF_YOUR_COMBAT,
            attach_to_target,
            "Celebration \u{2014} At the beginning of combat on your turn, if two or more nonland permanents entered the battlefield under your control this turn, attach this Equipment to up to one target creature you control.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL.optional()])
        .condition(history::you_celebrate),
        equip(mana("{2}"), "Equip {2}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<2, 0>, "Equipped creature gets +2/+0.")],
    ..oracle::BESPOKE_BATTLEGARB
};

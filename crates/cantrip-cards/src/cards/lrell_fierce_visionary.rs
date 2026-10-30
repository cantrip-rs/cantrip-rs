use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, targets::filters, tokens, triggers};
use cantrip_mechanics::tokens::KLINGON;
use cantrip_oracle as oracle;

/// "At the beginning of combat on your turn, create a 2/1 red Klingon Warrior creature token with haste. Sacrifice it at
/// the beginning of the next end step." The token is carried to its delayed trigger by object identity (CR 603.7c, 400.7).
fn create_klingon(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let created = tokens::create(g, item.controller, &KLINGON, 1)?;
    delayed::at_next_end_step_about(g, item, 0, &created);
    Ok(())
}

/// "Surveil 1." (CR 701.25a).
fn surveil_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::BEGINNING_OF_YOUR_COMBAT,
            create_klingon,
            "At the beginning of combat on your turn, create a 2/1 red Klingon Warrior creature token with haste. Sacrifice it at the beginning of the next end step.",
        ),
        ability(
            cost("{2}").and(sacrifice(1, filters::another_creature_or_artifact)),
            surveil_one,
            "{2}, Sacrifice another creature or artifact: Surveil 1.",
        ),
    ],
    delayed: &[delayed::SACRIFICE_CREATED],
    ..oracle::LRELL_FIERCE_VISIONARY
};

pub static EXTRA: &[&CardDef] = &[&KLINGON];

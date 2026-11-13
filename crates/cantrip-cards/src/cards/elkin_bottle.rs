use cantrip_api::*;
use cantrip_mechanics::library;
use cantrip_oracle as oracle;

/// Exile the top card and permit the ability controller to play it until their next upkeep (CR 406.2, 611.2a).
fn exile_top_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::exile_top_may_play(
        g,
        item.controller,
        1,
        item.controller,
        PlayGrant { lands: true, expiry: Expiry::UntilYourNextStep(Step::Upkeep), ..PlayGrant::CAST },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}").and(TAP),
        exile_top_card,
        "{3}, {T}: Exile the top card of your library. Until the beginning of your next upkeep, you may play that card.",
    )],
    ..oracle::ELKIN_BOTTLE
};

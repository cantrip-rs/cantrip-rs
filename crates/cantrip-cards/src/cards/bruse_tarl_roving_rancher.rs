use cantrip_api::*;
use cantrip_mechanics::{lands, statics, tokens, triggers};
use cantrip_oracle as oracle;

/// "Exile the top card of your library. If it's a land card, create a 2/2 white Ox creature token. Otherwise, you may cast it
/// until the end of your next turn" (CR 406, 601.3, 514.2): the card is judged as it was exiled, so a land sent elsewhere by a
/// replacement still makes the Ox; a nonland card may be cast, not played, only if it really reached exile. An empty library
/// exiles nothing, and nothing else happens.
fn exile_top_ox_or_cast(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(&top) = g.state().zone(Zone::library(item.controller)).first() else { return Ok(()) };
    let exiled = g.exile(top);
    if g.chars(exiled).is(CardType::Land) {
        tokens::create(g, item.controller, &tokens::OX, 1)?;
    } else if g.obj(exiled).zone.is(ZoneKind::Exile) {
        g.allow_play(exiled, item.controller, PlayGrant { instead: None, lands: false, expiry: Expiry::EndOfYourNextTurn, ..PlayGrant::CAST });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS_OR_ATTACKS,
        exile_top_ox_or_cast,
        "Whenever Bruse Tarl enters or attacks, exile the top card of your library. If it's a land card, create a 2/2 white Ox creature token. Otherwise, you may cast it until the end of your next turn.",
    )],
    statics: &[static_ability(Layer::Ability, statics::your_of_subtype::<{ lands::sub(Subtype::Ox) }>, statics::have_double_strike, "Oxen you control have double strike.")],
    ..oracle::BRUSE_TARL_ROVING_RANCHER
};

pub static EXTRA: &[&CardDef] = &[&tokens::OX];

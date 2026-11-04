use cantrip_api::*;
use cantrip_mechanics::statics::{artifacts_you_control, plus_pt};
use cantrip_mechanics::{library, select};
use cantrip_oracle as oracle;

/// "This creature gets +1/+0 as long as you control another artifact" (CR 613.4c): itself, while its controller has an artifact
/// besides it, as the earlier layers left the permanents (itself being one).
fn itself_with_another_artifact(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    affected == source && artifacts_you_control(view, source) - i16::from(view.chars(source).is(CardType::Artifact)) >= 1
}

/// "Search your library for a card named Magitek Infantry, put it onto the battlefield tapped, then shuffle" (CR 701.23): the
/// card may go unfound (CR 701.23b); the library is shuffled either way.
fn fetch(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rule = select::cards(select::up_to(1)).with(select::named(g.card_id(&CARD)));
    library::find_to_battlefield(g, item.controller, item.object, rule, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}{W}"), fetch, "{2}{W}: Search your library for a card named Magitek Infantry, put it onto the battlefield tapped, then shuffle.")],
    statics: &[static_ability(Layer::PtModify, itself_with_another_artifact, plus_pt::<1, 0>, "This creature gets +1/+0 as long as you control another artifact.")],
    ..oracle::MAGITEK_INFANTRY
};

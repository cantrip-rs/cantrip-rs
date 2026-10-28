use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Return this card and target land card from your graveyard to the battlefield tapped": both at once, under its
/// controller's control (CR 110.2a). This card is not returned if it is no longer the card that was in the graveyard
/// (CR 400.7); the land is returned if it is still a legal target (a spell or ability whose only target is gone does not
/// resolve, CR 608.2b, so it is here).
fn return_both(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let mut cards = vec![(item.object, you, true)];
    cards.extend(targets::object(g, item, 0).map(|land| (land, you, true)));
    g.put_all_onto_battlefield_as(&cards)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    keywords: Keywords::of(&[Keyword::CantBeBlockedByPowerTwoOrLess]),
    statics: &[characteristic_defining(
        Layer::PtDefine,
        lands::power_and_toughness_are_lands_you_control,
        "Sandman's power and toughness are each equal to the number of lands you control.",
    )
    .lost_pt(0, 0)],
    abilities: &[ability(
        cost("{3}{G}{G}"),
        return_both,
        "{3}{G}{G}: Return this card and target land card from your graveyard to the battlefield tapped.",
    )
    .from(ZoneKind::Graveyard)
    .targets(&[targets::LAND_CARD_IN_YOUR_GRAVEYARD])],
    ..oracle::SANDMAN_SHIFTING_SCOUNDREL
};

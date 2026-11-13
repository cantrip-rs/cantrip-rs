use cantrip_api::*;
use cantrip_mechanics::{effects, select, targets, triggers};
use cantrip_oracle as oracle;

/// "Daretti's power is equal to the greatest mana value among artifacts you control" (CR 604.3, 202.3, layer 7a): a
/// characteristic-defining ability. Daretti is not an artifact himself, so with none he is 0/5; {X} in an artifact's cost counts
/// as 0 on the battlefield (CR 107.3). The controller of each is read through the view (CR 613.1b).
fn power_is_greatest_artifact_mana_value_you_control(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let controller = q.controller(source);
    let greatest = q
        .state()
        .battlefield()
        .iter()
        .filter(|&&id| q.controller(id) == controller)
        .filter_map(|&id| {
            let c = q.chars(id);
            c.is(CardType::Artifact).then(|| q.mana_value(id, &c))
        })
        .max()
        .unwrap_or(0);
    chars.power = Some(greatest as i32);
}

/// "Choose target artifact card in your graveyard. You may sacrifice an artifact. If you do, return the chosen card to the
/// battlefield." The target is chosen as the ability goes on the stack, the sacrifice as it resolves (CR 608.2d, 701.21a); with
/// the chosen card gone from the graveyard the ability does not resolve at all (CR 608.2b), so nothing is offered. The artifact
/// sacrificed is not the chosen card, which is in a graveyard.
fn sacrifice_to_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !effects::may_sacrifice(g, item.controller, item.object, select::artifact_cards(select::up_to(1)))?.is_empty() {
        effects::return_first_target_to_battlefield(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    statics: &[characteristic_defining(
        Layer::PtDefine,
        power_is_greatest_artifact_mana_value_you_control,
        "Daretti's power is equal to the greatest mana value among artifacts you control.",
    )
    .lost_pt(0, 5)],
    abilities: &[triggered(
        triggers::THIS_ENTERS_OR_ATTACKS,
        sacrifice_to_return,
        "Whenever Daretti enters or attacks, choose target artifact card in your graveyard. You may sacrifice an artifact. If you do, return the chosen card to the battlefield.",
    )
    .targets(&[targets::ARTIFACT_CARD_IN_YOUR_GRAVEYARD])],
    ..oracle::DARETTI_ROCKETEER_ENGINEER
};

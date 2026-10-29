use cantrip_api::*;
use cantrip_mechanics::attach::equipment_controlled;
use cantrip_mechanics::{choose, effects, select};
use cantrip_oracle as oracle;

/// "Gets +2/+0 for each Equipment attached to him" (CR 613.4c, layer 7c; CR 301.5a): the Equipment as the earlier layers made
/// them, whoever controls them, attached to the source.
fn plus_two_power_per_equipment_attached(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let state = view.state();
    let n = state.battlefield().iter().filter(|&&id| state.obj(id).attached_to == Some(source) && view.chars(id).has_subtype(Subtype::Equipment)).count() as i16;
    chars.power = chars.power.map(|v| v + 2 * n);
}

/// "Return this card from your graveyard to the battlefield with a finality counter on him. Then you may attach an Equipment you
/// control to him": the choice is made as the ability resolves, from the Equipment its controller controls then (CR 701.3a);
/// one that cannot be attached stays where it is (CR 701.3b).
fn return_and_may_equip(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let returned = effects::put_this_card_onto_battlefield_with_finality(g, item)?;
    let equipment = equipment_controlled(g, item.controller);
    for attachment in choose::select(g, item.controller, item.object, Purpose::Attach, equipment, select::cards(select::up_to(1)))? {
        g.attach(attachment, returned);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}{W}{B}"),
        return_and_may_equip,
        "{3}{W}{B}: Return this card from your graveyard to the battlefield with a finality counter on him. Then you may attach an Equipment you control to him.",
    )
    .from(ZoneKind::Graveyard)],
    statics: &[static_ability(
        Layer::PtModify,
        cantrip_mechanics::statics::itself,
        plus_two_power_per_equipment_attached,
        "Winter Soldier gets +2/+0 for each Equipment attached to him.",
    )],
    ..oracle::WINTER_SOLDIER_ICY_ASSASSIN
};

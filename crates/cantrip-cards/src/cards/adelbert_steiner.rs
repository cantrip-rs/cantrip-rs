use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// "Adelbert Steiner gets +1/+1 for each Equipment you control" (CR 613.4c, layer 7c): attached or not, creature or not.
fn plus_one_plus_one_per_equipment_you_control(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    statics::plus_one_plus_one_for_each_you_control(view, source, chars, &|c| c.has_subtype(Subtype::Equipment));
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        statics::itself,
        plus_one_plus_one_per_equipment_you_control,
        "Adelbert Steiner gets +1/+1 for each Equipment you control.",
    )],
    ..oracle::ADELBERT_STEINER
};

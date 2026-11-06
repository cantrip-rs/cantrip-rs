use cantrip_api::*;
use cantrip_mechanics::{attach, chosen_type, targets};
use cantrip_oracle as oracle;

/// "As this Aura enters, choose a basic land type. Enchanted land is the chosen type." (CR 614.1c).
fn as_aura_enters_choose_basic_enters(
    g: &mut dyn Game,
    entering: &mut Entering,
) -> Result<(), Suspend> {
    chosen_type::choose_type_among(
        g,
        entering,
        vec![
            Subtype::Plains, Subtype::Island, Subtype::Swamp, Subtype::Mountain,
            Subtype::Forest
        ],
    )?;
    Ok(())
}

/// "As this Aura enters, choose a basic land type. Enchanted land is the chosen type." (CR 604.1).
fn as_aura_enters_choose_basic_apply_0(
    q: &dyn LayerQuery,
    source: ObjectId,
    chars: &mut Chars,
) {
    let asker = &Asker::new(source, q.controller(source));
    let Some(chosen) = q.state().chosen_subtype(q.linked_to(asker.source)) else { return };
    chars.set_land_subtypes(SubtypeSet::NONE.with(chosen));
}

pub static CARD: CardDef = CardDef {
    statics: &[
        sets_land_subtypes(
            attach::attached_object,
            as_aura_enters_choose_basic_apply_0,
            "Enchanted land is the chosen type.",
        ),
    ],
    enters: Some(as_aura_enters_choose_basic_enters),
    enchant: Some(targets::LAND),
    ..oracle::PHANTASMAL_TERRAIN
};

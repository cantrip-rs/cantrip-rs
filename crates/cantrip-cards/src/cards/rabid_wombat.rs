use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// "This creature gets +2/+2 for each Aura attached to it." (CR 604.1).
fn creature_gets_2_2_aura_apply_0(
    q: &dyn LayerQuery,
    source: ObjectId,
    chars: &mut Chars,
) {
    let aura = q.words(source).subtype(Subtype::Aura);
    let auras = q.state().battlefield().iter().filter(|&&id| {
        q.state().obj(id).attached_to == Some(source) && q.chars(id).has_subtype(aura)
    }).count() as i32;
    let bonus = auras * 2;
    chars.power = chars.power.map(|v| v + bonus);
    chars.toughness = chars.toughness.map(|v| v + bonus);
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::PtModify,
            statics::itself,
            creature_gets_2_2_aura_apply_0,
            "This creature gets +2/+2 for each Aura attached to it.",
        ),
    ],
    ..oracle::RABID_WOMBAT
};

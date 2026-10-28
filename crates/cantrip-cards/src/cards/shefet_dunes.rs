use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Sacrifice a Desert": any Desert the controller has, this land included (a 2017-07-14 ruling), read through the
/// words of this card's text (CR 612).
fn is_desert(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Desert))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        mana_ability(TAP.and(life(1)), add(Mana::W), "{T}, Pay 1 life: Add {W}."),
        ability(
            cost("{2}{W}{W}, {T}").and(sacrifice(1, is_desert)),
            effects::creatures_you_control_get::<1, 1>,
            "{2}{W}{W}, {T}, Sacrifice a Desert: Creatures you control get +1/+1 until end of turn. Activate only as a sorcery.",
        )
        .sorcery_speed(),
    ],
    ..oracle::SHEFET_DUNES
};

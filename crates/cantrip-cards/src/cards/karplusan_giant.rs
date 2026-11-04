use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Tap an untapped snow land you control: This creature gets +1/+1 until end of turn." (CR 608.2).
fn creature_gets_1_1_2(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    !(q.state().obj(id).tapped) && q.chars(id).supertypes.snow
        && q.chars(id).is(CardType::Land) && asker.controller == q.controller(id)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            tap_untapped(1, creature_gets_1_1_2),
            effects::this_gets::<1, 1>,
            "Tap an untapped snow land you control: This creature gets +1/+1 until end of turn.",
        ),
    ],
    ..oracle::KARPLUSAN_GIANT
};

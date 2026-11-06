use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Pestilence Rats's power is equal to the number of other Rats on the battlefield. (For example, as long as there are two other Rats on the battlefield, Pestilence Rats's power and toughness are 2/3.)" (CR 604.1).
fn pestilence_ratss_power_equal_number_apply_0(
    q: &dyn LayerQuery,
    source: ObjectId,
    chars: &mut Chars,
) {
    let asker = &Asker::new(source, q.controller(source));
    chars.power = Some(
        (q
            .state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                !(asker.source == id)
                    && q
                        .chars(id)
                        .has_subtype(q.words(asker.source).subtype(Subtype::Rat))
            })
            .collect::<Vec<ObjectId>>()
            .len() as i64) as i32,
    );
}

pub static CARD: CardDef = CardDef {
    power: Some(0),
    toughness: Some(3),
    statics: &[
        characteristic_defining(
            Layer::PtDefine,
            pestilence_ratss_power_equal_number_apply_0,
            "Pestilence Rats's power is equal to the number of other Rats on the battlefield.",
        ),
    ],
    ..oracle::PESTILENCE_RATS
};

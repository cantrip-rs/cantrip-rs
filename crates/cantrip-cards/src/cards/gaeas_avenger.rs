use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Gaea's Avenger's power and toughness are each equal to 1 plus the number of artifacts your opponents control." (CR 604.1).
fn gaeas_avengers_power_toughness_equal_apply_0(
    q: &dyn LayerQuery,
    source: ObjectId,
    chars: &mut Chars,
) {
    let asker = &Asker::new(source, q.controller(source));
    let (p, t) = (
        (1
            + q
                .state()
                .battlefield()
                .iter()
                .copied()
                .filter(|&id| {
                    q.chars(id).is(CardType::Artifact)
                        && q
                            .state()
                            .opponents(asker.controller)
                            .collect::<Vec<PlayerId>>()
                            .contains(&q.controller(id))
                })
                .collect::<Vec<ObjectId>>()
                .len() as i64) as i32,
        (1
            + q
                .state()
                .battlefield()
                .iter()
                .copied()
                .filter(|&id| {
                    q.chars(id).is(CardType::Artifact)
                        && q
                            .state()
                            .opponents(asker.controller)
                            .collect::<Vec<PlayerId>>()
                            .contains(&q.controller(id))
                })
                .collect::<Vec<ObjectId>>()
                .len() as i64) as i32,
    );
    chars.power = Some(p);
    chars.toughness = Some(t);
}

pub static CARD: CardDef = CardDef {
    power: Some(1),
    toughness: Some(1),
    statics: &[
        characteristic_defining(
            Layer::PtDefine,
            gaeas_avengers_power_toughness_equal_apply_0,
            "Gaea's Avenger's power and toughness are each equal to 1 plus the number of artifacts your opponents control.",
        ),
    ],
    ..oracle::GAEAS_AVENGER
};

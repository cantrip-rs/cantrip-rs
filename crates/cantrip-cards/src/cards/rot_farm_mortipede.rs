use cantrip_api::*;
use cantrip_mechanics::{pump, triggers};
use cantrip_oracle as oracle;

/// "This creature gets +1/+0 and gains menace and lifelink until end of turn" (CR 611.2a, 613.4c, 702.111, 702.15): one
/// instruction of continuous effects from the ability's source, the keywords in one layer 6 effect, none of which does anything
/// once it has left the battlefield (CR 400.7).
fn gets_and_gains_menace_and_lifelink(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.one_instruction(&mut |g| {
        pump(g, item.object, item.object, 1, 0);
        if g.is_on_battlefield(item.object) {
            let keywords = Keywords::of(&[Keyword::Menace, Keyword::Lifelink]);
            g.add_effect(item.object, item.object, EffectKind::AddKeywords(keywords), Duration::EndOfTurn);
        }
        Ok(())
    })
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::creature_card_leaves_your_graveyard),
        gets_and_gains_menace_and_lifelink,
        "Whenever one or more creature cards leave your graveyard, this creature gets +1/+0 and gains menace and lifelink until end of turn.",
    )
    .once_per_batch()],
    ..oracle::ROT_FARM_MORTIPEDE
};

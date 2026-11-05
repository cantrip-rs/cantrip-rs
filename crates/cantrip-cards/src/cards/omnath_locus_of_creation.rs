use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// What the landfall does depends on how many times this ability of this Omnath has resolved this turn before
/// (CR 608.2m): this turn's history counts them. First, gain 4 life. Second, add {R}{G}{W}{U}. Third, 4 damage to each
/// opponent and each planeswalker you don't control. A fourth and later resolution does nothing.
fn landfall(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let controller = item.controller;
    match g.history().resolutions(item.object, item.ability) {
        0 => g.gain_life(controller, 4),
        1 => {
            for mana in [Mana::R, Mana::G, Mana::W, Mana::U] {
                g.add_mana(controller, mana, 1);
            }
        }
        2 => {
            let mut hits: Vec<(ObjectId, Target, i64)> =
                g.state().opponents(controller).map(|p| (item.object, Target::Player(p), 4)).collect();
            let walkers: Vec<ObjectId> = g
                .state()
                .battlefield()
                .iter()
                .copied()
                .filter(|&id| g.controller(id) != controller && g.chars(id).is(CardType::Planeswalker))
                .collect();
            hits.extend(walkers.into_iter().map(|walker| (item.object, Target::Object(walker), 4)));
            g.damage_together(&hits);
        }
        _ => {}
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, |g, item| effects::draw(g, item, 1), "When Omnath enters, draw a card."),
        triggered(when_asked(EventMask::ZONE_CHANGE, triggers::landfall), landfall, "Landfall \u{2014} Whenever a land you control enters, you gain 4 life if this is the first time this ability has resolved this turn. If it's the second time, add {R}{G}{W}{U}. If it's the third time, Omnath deals 4 damage to each opponent and each planeswalker you don't control."),
    ],
    ..oracle::OMNATH_LOCUS_OF_CREATION
};

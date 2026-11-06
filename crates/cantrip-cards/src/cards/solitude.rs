use cantrip_api::*;
use cantrip_mechanics::{evoke, targets};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

fn white_card(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).colors.has(q.words(asker.source).color(Color::White))
}

/// Exile up to one other target creature; its controller gains life equal to its power, as it last existed.
fn exile_and_compensate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        let power = g.chars(creature).power.unwrap_or(0) as i32;
        let controller = g.controller(creature);
        g.exile(creature);
        if power > 0 {
            g.gain_life(controller,(power) as i32);
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        evoke::sacrifice_trigger("When Solitude enters, if its evoke cost was paid, its controller sacrifices it."),
        triggered(THIS_ENTERS, exile_and_compensate, "When Solitude enters, exile up to one other target creature. That creature's controller gains life equal to its power.")
            .targets(&[up_to_one(targets::other_creature_filter)]),
    ],
    // Evoke: exile a white card from your hand rather than pay this spell's mana cost (CR 118.9).
    ways: &[evoke::way(evoke::exile_from_hand(white_card), "Evoke - Exile a white card from your hand")],
    ..oracle::SOLITUDE
};

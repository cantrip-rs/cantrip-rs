use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// Every creature and every Vehicle on the battlefield, in battlefield order (CR 205.3, 301.7): a Vehicle is a permanent with the
/// subtype whether or not it is a creature at the moment, read through the words of the spell's text (CR 612.2).
fn creatures_and_vehicles(g: &dyn Game, spell: ObjectId) -> Vec<ObjectId> {
    let vehicle = g.words(spell).subtype(Subtype::Vehicle);
    g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Creature) || g.chars(id).has_subtype(vehicle)).collect()
}

/// "All creatures and Vehicles lose indestructible until end of turn, then destroy all creatures and Vehicles." (CR 702.12,
/// 701.8a, 608.2c, 611.2c): the first instruction is a continuous effect on exactly the creatures and Vehicles there are as it
/// resolves (CR 611.2c), applied before the next instruction looks at them (CR 608.2c), so all of them can be destroyed at once.
fn lose_indestructible_then_destroy_all(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let permanents = creatures_and_vehicles(g, item.object);
    instruction(g, |g| {
        for &permanent in &permanents {
            g.add_effect(item.object, permanent, EffectKind::LoseKeywords(Keywords::of(&[Keyword::Indestructible])), Duration::EndOfTurn);
        }
    });
    g.end_sentence();
    let doomed = creatures_and_vehicles(g, item.object);
    g.destroy_all(&doomed);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], lose_indestructible_then_destroy_all),
    abilities: &[triggers::cycling(mana("{2}"), "Cycling {2} ({2}, Discard this card: Draw a card.)")],
    ..oracle::SPECTACULAR_PILEUP
};

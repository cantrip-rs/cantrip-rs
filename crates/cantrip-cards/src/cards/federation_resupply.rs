use cantrip_api::*;
use cantrip_mechanics::{beam_me_up, library, select};
use cantrip_oracle as oracle;

/// "An Aura, Equipment, or Spacecraft card" (CR 701.23, 701.20): each type is read through this spell's words
/// (CR 612.2), and a search for stated qualities may find none (CR 701.23b).
fn aura_equipment_or_spacecraft(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let words = q.words(choice.source);
    let aura = words.subtype(Subtype::Aura);
    let equipment = words.subtype(Subtype::Equipment);
    let spacecraft = words.subtype(Subtype::Spacecraft);
    Verdict::each(choice, |card| {
        let chars = q.chars(card);
        chars.has_subtype(aura) || chars.has_subtype(equipment) || chars.has_subtype(spacecraft)
    })
}

/// Search your library for an Aura, Equipment, or Spacecraft card, reveal it, put it into your hand, then shuffle.
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rule = Selection::new(select::up_to(1))
        .with(select::part(&(aura_equipment_or_spacecraft as SelectFn), 0, ""))
        .called("Aura, Equipment, or Spacecraft card");
    library::find_to_hand(g, item.controller, item.object, rule, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], search),
    ways: &[beam_me_up::way(mana("{3}{W}"), "Beam me up {3}{W}")],
    ..oracle::FEDERATION_RESUPPLY
};

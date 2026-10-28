use cantrip_api::*;
use cantrip_mechanics::{flashback, tokens};
use cantrip_oracle as oracle;

/// "Tap three untapped creatures you control" (CR 118.3, 702.34a): any untapped creatures its controller controls, one that
/// came under their control this turn included (CR 302.6 limits only a creature's own {T} abilities; this is the tapping of
/// permanents as a cost, not the {T} symbol).
fn untapped_creature_you_control(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.controller(id) == asker.controller && !q.obj(id).tapped && q.chars(id).is(CardType::Creature)
}

/// "Create a 2/2 red and white Spirit creature token."
fn spirit(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::SPIRIT_2_2_RED_WHITE, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], spirit),
    ways: &[flashback::way(
        tap_untapped(3, untapped_creature_you_control),
        "Flashback\u{2014}Tap three untapped creatures you control. (You may cast this card from your graveyard for its flashback cost. Then exile it.)",
    )],
    ..oracle::GROUP_PROJECT
};

pub static EXTRA: &[&CardDef] = &[&tokens::SPIRIT_2_2_RED_WHITE];

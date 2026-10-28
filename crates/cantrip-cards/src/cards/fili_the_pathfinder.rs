use cantrip_api::*;
use cantrip_mechanics::{statics, storied, tokens};
use cantrip_oracle as oracle;

pub static EXTRA: &[&CardDef] = &[&tokens::DWARF];

/// "As long as you have an enduring story, creatures you control get +1/+1" (CR 702.195b, 613.4c): every creature its
/// controller controls, itself among them, while that player has the designation.
fn your_creatures_with_an_enduring_story(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    storied::you_have_an_enduring_story(q.state(), source) && statics::your_creatures(q, source, affected, so_far)
}

/// "Whenever Fíli or another nontoken Dwarf you control enters" (CR 603.6a): this permanent, token or not, or another
/// permanent that is a Dwarf and no token, under your control as it entered. "Dwarf" is a word of its text (CR 612).
fn fili_or_another_nontoken_dwarf_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if to.is(ZoneKind::Battlefield)
                && (*new == me.source
                    || (q.controller(*new) == me.controller
                        && !q.obj(*new).token
                        && q.chars(*new).has_subtype(q.words(me.source).subtype(Subtype::Dwarf)))) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "Create a 2/2 red Dwarf creature token."
fn dwarf(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::DWARF, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    storied: true,
    statics: &[static_ability(Layer::PtModify, your_creatures_with_an_enduring_story, statics::plus_pt::<1, 1>, "As long as you have an enduring story, creatures you control get +1/+1.")],
    abilities: &[triggered(when_asked(EventMask::ZONE_CHANGE, fili_or_another_nontoken_dwarf_enters), dwarf, "Whenever Fíli or another nontoken Dwarf you control enters, create a 2/2 red Dwarf creature token.")],
    ..oracle::F_LI_THE_PATHFINDER
};

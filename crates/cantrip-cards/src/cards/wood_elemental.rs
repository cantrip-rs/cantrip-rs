use cantrip_api::*;
use cantrip_mechanics::{choose, select};
use cantrip_oracle as oracle;

fn untapped_forest(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let forest = q.words(choice.source).subtype(Subtype::Forest);
    Verdict::each(choice, |id| !q.obj(id).tapped && q.chars(id).has_subtype(forest))
}

/// "As this creature enters, sacrifice any number of untapped Forests. Wood Elemental's power and toughness are each equal to the number of Forests sacrificed as it entered." (CR 614.1c, 701.21a).
fn as_creature_enters_sacrifice_any_enters(
    g: &mut dyn Game,
    entering: &mut Entering,
) -> Result<(), Suspend> {
    let player = entering.controller;
    let candidates = g.permanents_of(player);
    let chosen = choose::select(
        g, player, entering.object, Purpose::Sacrifice, candidates,
        select::Selection::new(select::ANY_NUMBER)
            .with(select::part(&(untapped_forest as SelectFn), 0, "")),
    )?;
    g.sacrifice_as_enters(entering, player, &chosen);
    Ok(())
}

/// "As this creature enters, sacrifice any number of untapped Forests. Wood Elemental's power and toughness are each equal to the number of Forests sacrificed as it entered." (CR 604.3).
fn as_creature_enters_sacrifice_any_apply_0(
    q: &dyn LayerQuery,
    source: ObjectId,
    chars: &mut Chars,
) {
    let forest = q.words(source).subtype(Subtype::Forest);
    let n = (q.history().sacrificed_as_entered(q.state(), q.linked_to(source))
        .filter(|row| row.subtypes.has(forest)).count()) as i32;
    let (p, t) = (n, n);
    chars.power = Some(p);
    chars.toughness = Some(t);
}

pub static CARD: CardDef = CardDef {
    power: Some(0),
    toughness: Some(0),
    statics: &[
        characteristic_defining(
            Layer::PtDefine,
            as_creature_enters_sacrifice_any_apply_0,
            "Wood Elemental's power and toughness are each equal to the number of Forests sacrificed as it entered.",
        ),
    ],
    enters: Some(as_creature_enters_sacrifice_any_enters),
    ..oracle::WOOD_ELEMENTAL
};

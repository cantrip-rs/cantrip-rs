use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{history, targets};
use cantrip_oracle as oracle;

/// "When The Black Arrow enters, it deals 1 damage to any target. If a Dragon is dealt damage this way, destroy it." (CR 120.3,
/// 701.8a): the Dragons the Arrow actually dealt damage to are read from the damage events this resolution recorded, so damage
/// that was prevented names none and a redirection names whoever was dealt it (Redcap Melee). Dragon is a word of this card's
/// text (CR 612) and the permanent is a Dragon as the layers have it after the damage. The destruction is a second sentence, so
/// the damage is done first (CR 608.2c).
fn shoot(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let before = g.history().head() as usize;
    g.damage(item.object, item.target(0), 1);
    let dragon = g.words(item.object).subtype(Subtype::Dragon);
    let dragons: Vec<ObjectId> =
        history::permanents_dealt_damage_by(g.history(), item.object, before).into_iter().filter(|&id| g.chars(id).has_subtype(dragon)).collect();
    g.end_sentence();
    g.destroy_all(&dragons);
    Ok(())
}

/// "Equipped creature ... has reach" (CR 702.17a, 613.1f).
fn have_reach(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::Reach);
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, shoot, "When The Black Arrow enters, it deals 1 damage to any target. If a Dragon is dealt damage this way, destroy it.")
            .targets(&[targets::ANY]),
        equip(mana("{1}"), "Equip {1}"),
    ],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 1>, "Equipped creature gets +1/+1."),
        static_ability(Layer::Ability, attached_object, have_reach, "Equipped creature has reach."),
    ],
    ..oracle::THE_BLACK_ARROW
};

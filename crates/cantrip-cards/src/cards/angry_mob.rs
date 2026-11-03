use cantrip_api::*;
use cantrip_oracle as oracle;

// This single ability defines both values in every zone (CR 113.2c, 604.3, 613.4a).
fn mob_size(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let controller = q.controller(source);
    let swamps = if q.state().turn.active == controller {
        let swamp = q.words(source).subtype(Subtype::Swamp);
        q.state().battlefield().iter().filter(|&&id| {
            q.state().opponents(controller).any(|opponent| opponent == q.controller(id))
                && q.chars(id).has_subtype(swamp)
        }).count()
    } else {
        0
    };
    let size = (2 + swamps) as i32;
    chars.power = Some(size);
    chars.toughness = Some(size);
}

pub static CARD: CardDef = CardDef {
    statics: &[characteristic_defining(
        Layer::PtDefine,
        mob_size,
        "During your turn, Angry Mob's power and toughness are each equal to 2 plus the number of Swamps your opponents control. During turns other than yours, Angry Mob's power and toughness are each 2.",
    ).lost_pt(2, 2)],
    ..oracle::ANGRY_MOB
};

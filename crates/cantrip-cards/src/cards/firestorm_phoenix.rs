use cantrip_api::*;
use cantrip_mechanics::replacements::{owners_hand_instead_apply, this_would_die};
use cantrip_oracle as oracle;

/// "That card": the card this Firestorm Phoenix became as its replacement returned it to its owner's hand (CR 400.7), while
/// it is there. The effect's source is the permanent that would have died, and the history names the one object that
/// permanent became in a hand (the ruling: "The Firestorm Phoenix you return to your hand is tracked by its ability"), so a
/// different Firestorm Phoenix in that hand is no part of it, nor this one once it leaves the hand, a new object again.
fn that_card(q: &dyn Query, asker: &Asker, _: Target, id: ObjectId) -> bool {
    q.obj(id).zone.is(ZoneKind::Hand)
        && q.history()
            .select(EventQuery::kind(EventMask::ZONE_CHANGE).object(id).to(ZoneKind::Hand))
            .rev()
            .any(|r| matches!(r.event, Event::ZoneChange { old, new, .. } if old == asker.source && new == id))
}

/// "Until that player's next turn, that player plays with that card revealed in their hand and can't play it": rules of the
/// game about that card (CR 611.2c, 701.20a, 101.2), part of the replacement effect that returned it (CR 614.1a), begun as it
/// applies. "That player" is its owner, into whose hand it goes, whoever controlled the Phoenix: the rules last until that
/// player's next turn begins (CR 611.2a), and the one about playing binds that player alone (the words' `about`), so another
/// player an effect lets play the card (Sen Triplets) still may. Playing it is casting it or playing it as a land (CR
/// 701.18b); putting it onto the battlefield is no play (the ruling), so an effect that does still may.
fn revealed_and_unplayable(g: &mut dyn Game, asker: &Asker, _: &Proposal, _: &Proposal) {
    let me = g.card_id(&CARD);
    let owner = g.obj(asker.source).owner;
    instruction(g, |g| {
        for index in [REVEALED, CANT_PLAY] {
            g.add_player_effect(asker.controller, asker.source, EffectKind::LastingRule { card: me, index, about: Target::Player(owner) }, Duration::UntilNextTurnOf(owner));
        }
    });
}

const REVEALED: u8 = 0;
const CANT_PLAY: u8 = 1;

/// "If this creature would die, return it to its owner's hand instead" (CR 614.1a, 700.4): the death is the move from the
/// battlefield to a graveyard, which goes to its owner's hand in its place - the same event, so it leaves the battlefield
/// together with whatever else did (CR 614.6) and never dies (the ruling: "The 'rebirth' is a replacement effect").
pub static CARD: CardDef = CardDef {
    replacements: &[replacement(ReplacementKind::ZoneChange, this_would_die, owners_hand_instead_apply, "returns to its owner's hand instead of dying")
        .then(revealed_and_unplayable)],
    lasting_rules: &[
        LastingRule {
            text: "Until that player's next turn, that player plays with that card revealed in their hand.",
            rule: LastingRuleKind::RevealedInHand,
            applies: that_card,
        },
        LastingRule { text: "Until that player's next turn, that player can't play that card.", rule: LastingRuleKind::CantBePlayed, applies: that_card },
    ],
    ..oracle::FIRESTORM_PHOENIX
};

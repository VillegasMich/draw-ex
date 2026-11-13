# draw-ex

A live view of a physical _Yu-Gi-Oh!_ duel. Camera footage is shown with the cards that
DRAW 2 recognizes on the table, attributed to each player.

## Cards

**Passcode**:
The 8-digit number printed on a card, used as its identity everywhere. Leading zeros are
not significant.
_Avoid_: card ID, serial number, label

**Card data**:
The descriptive information about a passcode: name, type, stats and effect text, in each
supported language.
_Avoid_: card info, metadata

**Card image**:
The official artwork for a passcode, in one of three sizes: full, small or cropped.
_Avoid_: art, picture

**Deck list**:
The set of passcodes a player is known to be using, imported from a `.ydk` file.
Recognition prefers cards in the deck list.
_Avoid_: decklist, ydk

## Recognition

**Frame**:
One still image taken from a camera at a moment in time.
_Avoid_: image, snapshot

**Quad**:
The four corners of a card as seen in a frame, ordered top-left, top-right, bottom-right,
bottom-left of the card read upright.
_Avoid_: box, bounding box, OBB

**Detection**:
One card found in one frame: its quad, how sure the detector is that it is a card, its
candidates, and its recognition.
_Avoid_: result, prediction, hit

**Candidate**:
A passcode the classifier considers possible for a detection, with a score from 0 to 1.
_Avoid_: guess, label, class

**Recognition**:
The verdict on a single detection: recognized as one candidate, low confidence, not in
the deck list, or unreadable.
_Avoid_: status, classification

**Model variant**:
One of the interchangeable classifier builds (full, half or integer precision, or the
Yugiscan build), which trade accuracy against size and speed.
_Avoid_: precision, model version

**Model manifest**:
The list of model files that a given version of the recognition code requires, with
where to get them and how to verify them.
_Avoid_: model list, model config

## Duel

**Player**:
One of the two duelists, A or B.
_Avoid_: side, user, opponent

**Camera setup**:
How cameras map to players: one camera on one player, one camera on both players divided
by a split line, or two cameras with one per player.
_Avoid_: mode, layout

**Split line**:
A line drawn across a camera's view that separates Player A's half of the table from
Player B's.
_Avoid_: divider, midline

**Observation**:
A detection as the tracker receives it: a quad and candidates from a given camera at a
given time.
_Avoid_: detection (inside the tracker)

**Track**:
The continuing presence of one physical card on the table across many frames, built from
successive observations.
_Avoid_: object, instance

**Confirmed card**:
A track whose identity has gathered enough consistent evidence to be shown to viewers and
logged. Only confirmed cards reach the panels.
_Avoid_: recognized card, detected card

**Session**:
One duel, from when the operator starts it to when they end it.
_Avoid_: match, game, recording

**Duel log**:
The ordered record of a session's confirmed-card events, saved to disk.
_Avoid_: history, timeline file

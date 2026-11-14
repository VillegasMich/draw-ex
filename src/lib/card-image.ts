// Card images reach the webview from the local cache, never from YGOPRODeck (see
// CLAUDE.md, "YGOPRODeck usage"). Ticket I1 registers the `cardimg` protocol that
// serves these URLs; until then they do not load.
import type { ImageSize, Passcode } from './bindings';

/** The URL scheme card images are served under. */
export const CARD_IMAGE_SCHEME = 'cardimg';

/**
 * The URL of a card image: `cardimg://<size>/<passcode>`, e.g.
 * `cardimg://small/83764719`. The passcode is written as a plain integer.
 */
export function cardImageUrl(passcode: Passcode, size: ImageSize): string {
  return `${CARD_IMAGE_SCHEME}://${size}/${passcode}`;
}

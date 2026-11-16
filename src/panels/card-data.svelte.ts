// Card data for the panels and overlay labels, fetched once per (passcode, language)
// through `get_card` and cached for the app's lifetime.
import { SvelteMap } from 'svelte/reactivity';
import { commands, type Card, type Language, type Passcode } from '../lib/bindings';

/**
 * A card's data in the configured language. `undefined` while it loads, `null` when the
 * card is unknown or the lookup failed.
 */
export type CardLookup = Card | null | undefined;

class CardData {
  /** The language card names and text are shown in, from the settings. */
  language = $state<Language>('en');
  #cards = new SvelteMap<string, Card | null>();
  #pending = new Set<string>();

  /** The card's data in {@link language}, starting the lookup on first use. */
  get(passcode: Passcode): CardLookup {
    const language = this.language;
    const key = `${passcode}:${language}`;
    if (!this.#cards.has(key) && !this.#pending.has(key)) {
      this.#pending.add(key);
      void this.#load(passcode, language, key);
    }
    return this.#cards.get(key);
  }

  /** Reads the card language from the settings. Keeps English if that fails. */
  async loadLanguage(): Promise<void> {
    try {
      this.language = (await commands.getSettings()).language;
    } catch (error) {
      console.warn('Could not read the settings; showing cards in English.', error);
    }
  }

  async #load(passcode: Passcode, language: Language, key: string): Promise<void> {
    let card: Card | null = null;
    try {
      card = await commands.getCard(passcode, language);
    } catch (error) {
      console.warn(`Could not look up card ${passcode}.`, error);
    }
    this.#pending.delete(key);
    this.#cards.set(key, card);
  }
}

export const cardData = new CardData();

/** The name to show for a card: its name, or its passcode until (or unless) it is known. */
export function cardName(passcode: Passcode, card: CardLookup): string {
  return card?.name ?? `Card ${passcode}`;
}

/** The type line: card type, then attribute and monster type or Spell/Trap property. */
export function cardTypeLine(card: Card): string {
  const details = [card.attribute, card.race].filter((part) => part !== null).join(' ');
  return details ? `${card.kind} · ${details}` : card.kind;
}

/**
 * The stats line: level or rank or link rating, then ATK/DEF. `null` for Spells and
 * Traps, which have none.
 */
export function cardStats(card: Card): string | null {
  const stat = (value: number | null) => (value === null ? '?' : String(value));
  if (card.linkRating !== null) {
    return `LINK-${card.linkRating} · ATK ${stat(card.atk)}`;
  }
  if (card.level !== null) {
    const level = /xyz/i.test(card.kind) ? 'Rank' : 'Level';
    return `${level} ${card.level} · ATK ${stat(card.atk)} / DEF ${stat(card.def)}`;
  }
  return null;
}

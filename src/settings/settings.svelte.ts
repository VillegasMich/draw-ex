// The operator's settings as the backend stores them, for the Settings modal.
import { commands, type Settings } from '../lib/bindings';
import { errorMessage } from './format';

export class SettingsState {
  /** The stored settings, or `null` until they are loaded. */
  current = $state.raw<Settings | null>(null);
  /** Why the last load or save failed, if it did. */
  error = $state<string | null>(null);

  async load(): Promise<void> {
    try {
      this.current = await commands.getSettings();
      this.error = null;
    } catch (error) {
      this.error = errorMessage(error);
    }
  }

  /** The save in progress, so saves run one after another. */
  #saving: Promise<void> = Promise.resolve();

  /**
   * Saves the settings with `change` applied. The backend validates them. Saves run in
   * order, each applied on top of the previous one, so a quick second change can neither
   * overtake the first nor undo it.
   */
  update(change: Partial<Settings>): Promise<void> {
    this.#saving = this.#saving.then(async () => {
      if (!this.current) {
        return;
      }
      try {
        this.current = await commands.updateSettings({ ...this.current, ...change });
        this.error = null;
      } catch (error) {
        this.error = errorMessage(error);
      }
    });
    return this.#saving;
  }
}

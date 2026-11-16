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

  /** Saves the settings with `change` applied. The backend validates them. */
  async update(change: Partial<Settings>): Promise<void> {
    if (!this.current) {
      return;
    }
    try {
      this.current = await commands.updateSettings({ ...this.current, ...change });
      this.error = null;
    } catch (error) {
      this.error = errorMessage(error);
    }
  }
}

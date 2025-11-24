import { open } from "@tauri-apps/plugin-dialog";
import {
  writeTextFile,
  readTextFile,
  exists,
  remove,
  mkdir,
  BaseDirectory,
} from "@tauri-apps/plugin-fs";

const PREAMBLE_FILENAME = "preamble.tex";
const SETTINGS_FILENAME = "settings.json";

export class SettingsManager {
  private modal: HTMLDialogElement;
  private settingsSidebarItems: NodeListOf<HTMLButtonElement>;
  private settingsPages: NodeListOf<HTMLElement>;

  constructor() {
    this.modal = document.getElementById("settings-modal") as HTMLDialogElement;
    this.settingsSidebarItems = document.querySelectorAll(
      ".settings-sidebar-item"
    );
    this.settingsPages = document.querySelectorAll(".settings-panel");
    this.initListeners();
  }

  private initListeners() {
    // Modal open / clone
    const open_settings_btn = document.getElementById("open-settings-btn");
    const close_settings_btn = document.getElementById("close-settings-btn");

    open_settings_btn?.addEventListener("click", () => this.modal.showModal());
    close_settings_btn?.addEventListener("click", () => this.modal.close());

    // Preamble
    const import_preamble_btn = document.getElementById("import-preamble-btn");
    const reset_preamble_btn = document.getElementById("reset-preamble-btn");

    reset_preamble_btn?.addEventListener("click", () =>
      this.handlePreambleReset()
    );
    import_preamble_btn?.addEventListener("click", () =>
      this.handlePreambleImport()
    );

    // Open event
    window.addEventListener("open-settings", () => {
      this.modal.showModal();
      this.refreshStatus();
    });

    window.addEventListener("click", this.handleClickOutside);

    // Sidebar navigation
    this.settingsSidebarItems.forEach((btn) => {
      btn.addEventListener("click", () => {
        const targetId = btn.getAttribute("data-target");
        if (targetId) {
          this.switchPage(targetId, btn);
        }
      });
    });
  }

  private switchPage(targetId: string, activeBtn: HTMLButtonElement) {
    this.settingsSidebarItems.forEach((b) => b.classList.remove("active"));
    this.settingsPages.forEach((p) => p.classList.remove("active"));

    activeBtn.classList.add("active");

    const targetPage = document.getElementById(targetId);
    if (targetPage) {
      targetPage.classList.add("active");
    }
  }

  private async handlePreambleImport() {
    console.log("Importing preamble...");
    try {
      const selectedPath = await open({
        multiple: false,
        filters: [{ name: "LaTeX", extensions: ["tex"] }],
      });

      if (!selectedPath || typeof selectedPath !== "string") return;

      // Ensure directory exists, recursive = true fails silently if it does
      await mkdir("", { 
        baseDir: BaseDirectory.AppConfig, 
        recursive: true 
      });

      // Read source, write copy
      const content = await readTextFile(selectedPath);
      await writeTextFile(PREAMBLE_FILENAME, content, {
        baseDir: BaseDirectory.AppConfig,
      });

      this.refreshStatus();
      alert("Preamble updated successfully!");

      window.dispatchEvent(new Event("settings-changed"));
    } catch (err) {
      console.error(err);
      alert("Failed to import file.");
    }
  }

  private async handlePreambleReset() {
    console.log("Resetting preamble...");
    try {
      const existsPreamble = await exists(PREAMBLE_FILENAME, {
        baseDir: BaseDirectory.AppConfig,
      });
      if (existsPreamble) {
        await remove(PREAMBLE_FILENAME, {
          baseDir: BaseDirectory.AppConfig,
        });
      }

      this.refreshStatus();
      alert("Preamble reset to default.");
    } catch (err) {
      console.error(err);
      alert("Failed to reset preamble.");
    }
  }

  private handleClickOutside = (event: MouseEvent) => {
    if (event.target === this.modal) {
      this.modal.close();
    }
  }

public async refreshStatus() {
    const statusEl = document.getElementById("preamble-status");
    if (!statusEl) return;

    console.log("Refreshing preamble status...");

    const existsPreamble = await exists(PREAMBLE_FILENAME, {
      baseDir: BaseDirectory.AppConfig,
    });

    // Update Text
    statusEl.textContent = existsPreamble
      ? "Custom preamble loaded"
      : "Using default preamble";
    
    // Update Class for Styling
    // We toggle the 'status-active' class based on existence
    if (existsPreamble) {
      statusEl.classList.add("status-active");
    } else {
      statusEl.classList.remove("status-active");
    }

    console.log("Preamble status updated.");
  }
}
